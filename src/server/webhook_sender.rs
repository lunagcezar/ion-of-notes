//! Outbound webhook delivery adapter (spec-041).
//!
//! The `WebhookSender` port keeps the server testable without network access.
//! `UreqWebhookSender` uses rustls, a bounded timeout and no automatic
//! redirects (a redirect must not become an SSRF bypass).

use crate::webhooks::{is_private_ip, url_host};
use std::net::{SocketAddr, ToSocketAddrs};
use std::time::Duration;
use ureq::config::Config;
use ureq::http::Uri;
use ureq::unversioned::resolver::{ResolvedSocketAddrs, Resolver};
use ureq::unversioned::transport::{DefaultConnector, NextTimeout};
use ureq::{Agent, Error as UreqError};

/// Deliver one signed webhook payload.
pub trait WebhookSender: Send + Sync {
    /// POST `body` to `url` with `signature`; return the HTTP status.
    fn send(&self, url: &str, body: &str, signature: &str) -> Result<u16, String>;

    /// Allow private/loopback destinations for explicit development setups.
    fn allow_private_targets(&mut self, _allow: bool) {}
}

/// Production sender using ureq + rustls.
pub struct UreqWebhookSender {
    config: Config,
    allow_private: bool,
}

/// Resolver that returns exactly the addresses validated by the caller.
///
/// Binding the validated resolution to the connection closes the DNS-rebinding
/// gap between the pre-flight check and the actual request: the HTTP stack does
/// not perform a second lookup.
#[derive(Debug)]
struct PinnedResolver {
    addresses: Vec<SocketAddr>,
}

impl Resolver for PinnedResolver {
    fn resolve(
        &self,
        _uri: &Uri,
        _config: &Config,
        _timeout: NextTimeout,
    ) -> Result<ResolvedSocketAddrs, UreqError> {
        if self.addresses.is_empty() {
            return Err(UreqError::HostNotFound);
        }
        let mut resolved = ResolvedSocketAddrs::from_fn(|_| SocketAddr::from(([0, 0, 0, 0], 0)));
        for address in self.addresses.iter().take(16) {
            resolved.push(*address);
        }
        Ok(resolved)
    }
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
            config,
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
        let port = url_port(url).unwrap_or(if scheme.eq_ignore_ascii_case("https") {
            443
        } else {
            80
        });
        let addresses: Vec<SocketAddr> = (host, port)
            .to_socket_addrs()
            .map_err(|error| format!("webhook target could not be resolved: {error}"))?
            .collect();
        if addresses.is_empty() {
            return Err("webhook target did not resolve".to_string());
        }
        if !self.allow_private && addresses.iter().any(|address| is_private_ip(address.ip())) {
            return Err("webhook target resolves to a private address".to_string());
        }
        let agent = Agent::with_parts(
            self.config.clone(),
            DefaultConnector::new(),
            PinnedResolver { addresses },
        );
        let response = agent
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

/// The explicit port in a URL, when present. IPv6 authorities keep their
/// brackets, so the last colon only separates a port when the suffix parses.
fn url_port(url: &str) -> Option<u16> {
    let authority = url.split_once("://")?.1.split(['/', '?', '#']).next()?;
    authority.rsplit_once(':')?.1.parse().ok()
}
