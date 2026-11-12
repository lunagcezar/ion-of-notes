//! Outbound webhook delivery adapter (spec-041).
//!
//! The `WebhookSender` port keeps the server testable without network access.
//! `UreqWebhookSender` uses rustls, a bounded timeout and no automatic
//! redirects (a redirect must not become an SSRF bypass).

use crate::webhooks::{is_private_ip, url_host};
use std::net::ToSocketAddrs;
use std::time::Duration;
use ureq::Agent;

/// Deliver one signed webhook payload.
pub trait WebhookSender: Send + Sync {
    /// POST `body` to `url` with `signature`; return the HTTP status.
    fn send(&self, url: &str, body: &str, signature: &str) -> Result<u16, String>;

    /// Allow private/loopback destinations for explicit development setups.
    fn allow_private_targets(&mut self, _allow: bool) {}
}

/// Production sender using ureq + rustls.
pub struct UreqWebhookSender {
    agent: Agent,
    allow_private: bool,
}

impl UreqWebhookSender {
    pub fn new() -> Self {
        Self::with_private_targets(false)
    }

    /// Development sender that permits loopback/private targets.
    pub fn new_development() -> Self {
        Self::with_private_targets(true)
    }

    fn with_private_targets(allow_private: bool) -> Self {
        let config = Agent::config_builder()
            .timeout_global(Some(Duration::from_secs(5)))
            .max_redirects(0)
            .max_redirects_will_error(false)
            .http_status_as_error(false)
            .user_agent("ion-of-notes-webhooks/0.1")
            .build();
        Self {
            agent: config.into(),
            allow_private,
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
        let (scheme, host) = url_host(url).ok_or_else(|| "invalid webhook URL".to_string())?;
        let port = if scheme.eq_ignore_ascii_case("https") {
            443
        } else {
            80
        };
        let addresses = (host, port)
            .to_socket_addrs()
            .map_err(|error| format!("webhook target could not be resolved: {error}"))?;
        if !self.allow_private
            && addresses
                .into_iter()
                .any(|address| is_private_ip(address.ip()))
        {
            return Err("webhook target resolves to a private address".to_string());
        }
        let response = self
            .agent
            .post(url)
            .header("Content-Type", "application/json")
            .header("X-Ion-Of-Notes-Signature", signature)
            .send(body)
            .map_err(|error| error.to_string())?;
        Ok(response.status().as_u16())
    }

    fn allow_private_targets(&mut self, allow: bool) {
        self.allow_private = allow;
    }
}
