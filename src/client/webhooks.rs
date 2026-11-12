use super::*;

impl<A: RequestTransport> Client<A> {
    pub fn list_webhooks_v2(&mut self) -> Result<Vec<WebhookSubscription>, ClientError> {
        let request = self.v2_request(Method::Get, "/api/v2/webhooks", None)?;
        self.call_v2(request)
    }

    pub fn create_webhook_v2(
        &mut self,
        input: &WebhookSubscriptionInput,
    ) -> Result<WebhookSubscription, ClientError> {
        let body = encode_body(input)?;
        let request = self.v2_request(Method::Post, "/api/v2/webhooks", Some(body))?;
        self.call_v2(request)
    }

    pub fn verify_webhook_v2(&mut self, id: &str, token: &str) -> Result<String, ClientError> {
        let body = encode_body(&WebhookVerificationInput {
            verification_token: token.to_string(),
        })?;
        let request = self.v2_request(
            Method::Post,
            format!("/api/v2/webhooks/{id}/verify"),
            Some(body),
        )?;
        self.call_v2(request)
    }

    pub fn delete_webhook_v2(&mut self, id: &str) -> Result<String, ClientError> {
        let request = self.v2_request(Method::Delete, format!("/api/v2/webhooks/{id}"), None)?;
        self.call_v2(request)
    }

    pub fn list_webhook_deliveries_v2(&mut self) -> Result<Vec<WebhookDelivery>, ClientError> {
        let request = self.v2_request(Method::Get, "/api/v2/webhook_deliveries", None)?;
        self.call_v2(request)
    }

    pub fn retry_webhook_deliveries_v2(&mut self) -> Result<Vec<WebhookDelivery>, ClientError> {
        let request = self.v2_request(Method::Post, "/api/v2/webhook_deliveries/retry", None)?;
        self.call_v2(request)
    }
}
