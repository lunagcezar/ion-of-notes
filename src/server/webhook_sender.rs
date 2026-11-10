//! Outbound webhook delivery adapter (spec-041).
//!
//! The `WebhookSender` port keeps the server testable without network access.
//! `UreqWebhookSender` uses rustls, a bounded timeout and no automatic
//! redirects (a redirect must not become an SSRF bypass).

use std::time::Duration;
use ureq::Agent;

/// Deliver one signed webhook payload.
pub trait WebhookSender: Send + Sync {
    /// POST `body` to `url` with `signature`; return the HTTP status.
    fn send(&self, url: &str, body: &str, signature: &str) -> Result<u16, String>;
}

/// Production sender using ureq + rustls.
pub struct UreqWebhookSender {
    agent: Agent,
}

impl UreqWebhookSender {
    pub fn new() -> Self {
        let config = Agent::config_builder()
            .timeout_global(Some(Duration::from_secs(5)))
            .max_redirects(0)
            .max_redirects_will_error(false)
            .http_status_as_error(false)
            .user_agent("ion-of-notes-webhooks/0.1")
            .build();
        Self {
            agent: config.into(),
        }
    }
}

impl Default for UreqWebhookSender {
    fn default() -> Self {
        Self::new()
    }
}

impl WebhookSender for UreqWebhookSender {
    fn send(&self, url: &str, body: &str, signature: &str) -> Result<u16, String> {
        let response = self
            .agent
            .post(url)
            .header("Content-Type", "application/json")
            .header("X-Ion-Of-Notes-Signature", signature)
            .send(body)
            .map_err(|error| error.to_string())?;
        Ok(response.status().as_u16())
    }
}
