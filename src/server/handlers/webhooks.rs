//! Webhook subscription, delivery and v2 route use cases.

use super::super::contract::{CODE_INVALID_JSON, CODE_NOT_FOUND, CODE_VALIDATION_ERROR};
use super::super::response::{json, parse_body, v2_error};
use super::super::{ApiRequest, ApiResponse, Envelope, Method, WorkspaceServer};
use crate::model::now_iso;
use crate::webhooks::{
    EntityRef, SubscriptionStatus, WebhookDelivery, WebhookEvent, WebhookEventType,
    WebhookSubscription, WebhookSubscriptionInput, WebhookVerificationInput, aggregate_events,
    is_allowed_url, sign,
};

impl WorkspaceServer {
    pub fn list_webhooks(&self) -> &[WebhookSubscription] {
        &self.collab.webhooks
    }

    pub fn create_webhook(
        &mut self,
        input: WebhookSubscriptionInput,
    ) -> Result<WebhookSubscription, String> {
        if input.url.trim().is_empty() || input.events.is_empty() {
            return Err("url and at least one event are required".to_string());
        }
        if !is_allowed_url(&input.url, self.allow_insecure_webhooks) {
            return Err(
                "webhook URL must be HTTPS (loopback HTTP requires the development override)"
                    .to_string(),
            );
        }
        let subscription = WebhookSubscription {
            id: format!("wh-{}", uuid::Uuid::new_v4()),
            url: input.url,
            events: input.events,
            verification_token: format!("secret_{}", uuid::Uuid::new_v4().simple()),
            status: SubscriptionStatus::Pending,
            created_at: now_iso(),
        };
        self.collab.webhooks.push(subscription.clone());
        self.persist_collab().map_err(|error| error.to_string())?;
        Ok(subscription)
    }

    pub fn verify_webhook(&mut self, id: &str, token: &str) -> Result<(), String> {
        let Some(subscription) = self
            .collab
            .webhooks
            .iter_mut()
            .find(|subscription| subscription.id == id)
        else {
            return Err("webhook not found".to_string());
        };
        if subscription.verification_token != token {
            return Err("verification token does not match".to_string());
        }
        subscription.status = SubscriptionStatus::Active;
        self.persist_collab().map_err(|error| error.to_string())
    }

    pub fn delete_webhook(&mut self, id: &str) -> Result<(), String> {
        let before = self.collab.webhooks.len();
        self.collab
            .webhooks
            .retain(|subscription| subscription.id != id);
        if self.collab.webhooks.len() == before {
            return Err("webhook not found".to_string());
        }
        self.persist_collab().map_err(|error| error.to_string())
    }

    pub fn webhook_deliveries(&self) -> &[WebhookDelivery] {
        &self.collab.webhook_deliveries
    }

    /// Queue or deliver an event to active subscriptions.
    pub fn emit_webhook(
        &mut self,
        event_type: WebhookEventType,
        entity_type: &str,
        entity_id: &str,
        data: impl Into<String>,
    ) {
        let data = data.into();
        let actor = self.collab.owner_id();
        let workspace_id = self.workspace.settings.id.clone();
        let subscriptions: Vec<WebhookSubscription> = self
            .collab
            .webhooks
            .iter()
            .filter(|subscription| {
                subscription.status == SubscriptionStatus::Active
                    && subscription.events.contains(&event_type)
            })
            .cloned()
            .collect();
        for subscription in subscriptions {
            let event = WebhookEvent::new(
                workspace_id.clone(),
                subscription.id.clone(),
                event_type,
                EntityRef {
                    id: entity_id.to_string(),
                    r#type: entity_type.to_string(),
                },
                actor.clone(),
                data.clone(),
            );
            if event_type.is_aggregated() {
                if let Some(existing) = self.webhook_queue.iter_mut().find(|existing| {
                    existing.subscription_id == event.subscription_id
                        && existing.entity.id == event.entity.id
                        && existing.r#type == event.r#type
                }) {
                    *existing = event;
                } else {
                    self.webhook_queue.push(event);
                }
            } else {
                self.deliver_event(&subscription, event);
            }
        }
    }

    /// Flush aggregated events (the production worker calls this on a timer).
    pub fn flush_webhooks(&mut self) {
        let queued = std::mem::take(&mut self.webhook_queue);
        for event in aggregate_events(queued) {
            if let Some(subscription) = self
                .collab
                .webhooks
                .iter()
                .find(|subscription| subscription.id == event.subscription_id)
                .cloned()
            {
                self.deliver_event(&subscription, event);
            }
        }
    }

    /// Retry failed deliveries with a small exponential backoff.
    pub fn process_webhook_deliveries(&mut self) {
        let subscriptions: Vec<WebhookSubscription> = self.collab.webhooks.clone();
        for delivery in &mut self.collab.webhook_deliveries {
            if delivery.delivered || delivery.attempts >= 8 {
                continue;
            }
            let Some(subscription) = subscriptions
                .iter()
                .find(|subscription| subscription.id == delivery.subscription_id)
            else {
                continue;
            };
            let signature = sign(&subscription.verification_token, &delivery.payload);
            let result = self
                .webhook_sender
                .send(&subscription.url, &delivery.payload, &signature);
            delivery.attempts = delivery.attempts.saturating_add(1);
            match result {
                Ok(status) => {
                    delivery.last_status = Some(status);
                    delivery.last_error = None;
                    delivery.delivered = (200..300).contains(&status);
                    delivery.next_attempt_at = if delivery.delivered {
                        None
                    } else {
                        Some(backoff_time(delivery.attempts))
                    };
                }
                Err(error) => {
                    delivery.last_status = None;
                    delivery.last_error = Some(error);
                    delivery.next_attempt_at = Some(backoff_time(delivery.attempts));
                }
            }
        }
        let _ = self.persist_collab();
    }

    fn deliver_event(&mut self, subscription: &WebhookSubscription, mut event: WebhookEvent) {
        event.attempt_number = 1;
        let payload = serde_json::to_string(&event).unwrap_or_default();
        let signature = sign(&subscription.verification_token, &payload);
        let result = self
            .webhook_sender
            .send(&subscription.url, &payload, &signature);
        let delivered = result
            .as_ref()
            .is_ok_and(|status| (200..300).contains(status));
        self.collab.webhook_deliveries.push(WebhookDelivery {
            id: format!("delivery-{}", uuid::Uuid::new_v4()),
            subscription_id: subscription.id.clone(),
            event_id: event.id.clone(),
            payload,
            attempts: 1,
            last_status: result.as_ref().ok().copied(),
            last_error: result.err(),
            delivered,
            next_attempt_at: if delivered {
                None
            } else {
                Some(backoff_time(1))
            },
        });
        let _ = self.persist_collab();
    }

    pub(in crate::server) fn handle_v2_webhook(
        &mut self,
        request: &ApiRequest,
        path: &str,
        request_id: &str,
    ) -> Option<ApiResponse> {
        if path == "/api/v2/webhooks" && request.method == Method::Get {
            return Some(json(200, Envelope::ok(self.collab.webhooks.clone())));
        }
        if path == "/api/v2/webhooks" && request.method == Method::Post {
            let input = match parse_body::<WebhookSubscriptionInput>(request) {
                Ok(input) => input,
                Err(message) => {
                    return Some(v2_error(400, CODE_INVALID_JSON, message, request_id, None));
                }
            };
            return match self.create_webhook(input) {
                Ok(subscription) => Some(json(201, Envelope::ok(subscription))),
                Err(message) => Some(v2_error(
                    400,
                    CODE_VALIDATION_ERROR,
                    message,
                    request_id,
                    None,
                )),
            };
        }
        if path == "/api/v2/webhook_deliveries" && request.method == Method::Get {
            return Some(json(
                200,
                Envelope::ok(self.collab.webhook_deliveries.clone()),
            ));
        }
        if path == "/api/v2/webhook_deliveries/retry" && request.method == Method::Post {
            self.process_webhook_deliveries();
            return Some(json(
                200,
                Envelope::ok(self.collab.webhook_deliveries.clone()),
            ));
        }
        let rest = path.strip_prefix("/api/v2/webhooks/")?;
        if let Some(id) = rest.strip_suffix("/verify") {
            if request.method != Method::Post || id.is_empty() {
                return Some(v2_error(
                    404,
                    CODE_NOT_FOUND,
                    "webhook not found",
                    request_id,
                    None,
                ));
            }
            let input = match parse_body::<WebhookVerificationInput>(request) {
                Ok(input) => input,
                Err(message) => {
                    return Some(v2_error(400, CODE_INVALID_JSON, message, request_id, None));
                }
            };
            return match self.verify_webhook(id, &input.verification_token) {
                Ok(()) => Some(json(200, Envelope::ok("verified".to_string()))),
                Err(message) => Some(v2_error(
                    400,
                    CODE_VALIDATION_ERROR,
                    message,
                    request_id,
                    None,
                )),
            };
        }
        if request.method == Method::Delete && !rest.is_empty() && !rest.contains('/') {
            return match self.delete_webhook(rest) {
                Ok(()) => Some(json(200, Envelope::ok("deleted".to_string()))),
                Err(message) => Some(v2_error(404, CODE_NOT_FOUND, message, request_id, None)),
            };
        }
        None
    }
}

fn backoff_time(attempts: u8) -> String {
    let seconds = 2u64.saturating_pow(attempts as u32).min(3600);
    (chrono::Utc::now() + chrono::Duration::seconds(seconds as i64)).to_rfc3339()
}
