//! Webhook subscription, signature, aggregation and delivery tests (spec-036).

mod common;

use ion_of_notes::client::Client;
use ion_of_notes::model::Workspace;
use ion_of_notes::server::webhook_sender::UreqWebhookSender;
use ion_of_notes::server::{Method, WebhookSender, WorkspaceApi, WorkspaceServer};
use ion_of_notes::webhooks::{
    SubscriptionStatus, WebhookDelivery, WebhookEvent, WebhookEventType, WebhookSubscription,
    WebhookSubscriptionInput, sign, verify,
};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;

#[derive(Debug, Clone)]
struct Received {
    body: String,
    signature: String,
}

struct RecordingSender {
    calls: Arc<Mutex<Vec<(String, String, String)>>>,
}

impl WebhookSender for RecordingSender {
    fn send(&self, url: &str, body: &str, signature: &str) -> Result<u16, String> {
        self.calls
            .lock()
            .unwrap()
            .push((url.to_string(), body.to_string(), signature.to_string()));
        Ok(204)
    }
}

struct FailingSender;

impl WebhookSender for FailingSender {
    fn send(&self, _url: &str, _body: &str, _signature: &str) -> Result<u16, String> {
        Err("receiver is down".to_string())
    }
}

fn start_receiver(fail_first: bool) -> (u16, Arc<Mutex<Vec<Received>>>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let received = Arc::new(Mutex::new(Vec::new()));
    let store = Arc::clone(&received);
    thread::spawn(move || {
        let mut first = true;
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            let request = read_request(&mut stream);
            let (head, body) = request
                .split_once("\r\n\r\n")
                .unwrap_or((request.as_str(), ""));
            let signature = head
                .lines()
                .find(|line| {
                    line.to_ascii_lowercase()
                        .starts_with("x-ion-of-notes-signature:")
                })
                .and_then(|line| line.split_once(':'))
                .map(|(_, value)| value.trim().to_string())
                .unwrap_or_default();
            store.lock().unwrap().push(Received {
                body: body.to_string(),
                signature,
            });
            let status = if fail_first && first {
                first = false;
                "500 Internal Server Error"
            } else {
                "200 OK"
            };
            let _ = write!(
                stream,
                "HTTP/1.1 {status}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
            );
        }
    });
    (port, received)
}

fn read_request(stream: &mut TcpStream) -> String {
    let mut buffer = Vec::new();
    let mut chunk = [0u8; 4096];
    while let Ok(n) = stream.read(&mut chunk) {
        if n == 0 {
            break;
        }
        buffer.extend_from_slice(&chunk[..n]);
        let text = String::from_utf8_lossy(&buffer);
        if let Some((head, body)) = text.split_once("\r\n\r\n") {
            let length = head
                .lines()
                .find_map(|line| {
                    let (key, value) = line.split_once(':')?;
                    key.eq_ignore_ascii_case("content-length")
                        .then(|| value.trim().parse::<usize>().ok())
                        .flatten()
                })
                .unwrap_or(0);
            if body.len() >= length {
                break;
            }
        }
    }
    String::from_utf8_lossy(&buffer).to_string()
}

fn subscribe(
    server: &mut WorkspaceServer,
    url: &str,
    events: Vec<WebhookEventType>,
) -> WebhookSubscription {
    let subscription = server
        .create_webhook(WebhookSubscriptionInput {
            url: url.to_string(),
            events,
        })
        .unwrap();
    server
        .verify_webhook(&subscription.id, &subscription.verification_token)
        .unwrap();
    subscription
}

#[test]
fn subscription_create_verify_delete() {
    let mut server = WorkspaceServer::new(Workspace::seed("x"));
    server.allow_insecure_webhooks(true);
    let subscription = server
        .create_webhook(WebhookSubscriptionInput {
            url: "http://127.0.0.1:9/hook".to_string(),
            events: vec![WebhookEventType::PageCreated],
        })
        .unwrap();
    assert_eq!(subscription.status, SubscriptionStatus::Pending);
    assert!(!subscription.verification_token.is_empty());
    assert_eq!(server.list_webhooks().len(), 1);

    assert!(
        server
            .verify_webhook(&subscription.id, "wrong-token")
            .is_err()
    );
    server
        .verify_webhook(&subscription.id, &subscription.verification_token)
        .unwrap();
    assert_eq!(server.list_webhooks()[0].status, SubscriptionStatus::Active);

    server.delete_webhook(&subscription.id).unwrap();
    assert!(server.list_webhooks().is_empty());
}

#[test]
fn insecure_urls_require_the_development_override() {
    let mut server = WorkspaceServer::new(Workspace::seed("x"));
    let input = WebhookSubscriptionInput {
        url: "http://127.0.0.1:9/hook".to_string(),
        events: vec![WebhookEventType::PageCreated],
    };
    assert!(server.create_webhook(input.clone()).is_err());

    server.allow_insecure_webhooks(true);
    assert!(server.create_webhook(input).is_ok());

    let https = WebhookSubscriptionInput {
        url: "https://example.com/hook".to_string(),
        events: vec![WebhookEventType::PageCreated],
    };
    assert!(server.create_webhook(https).is_ok());
}

#[test]
fn injected_webhook_sender_receives_the_delivery() {
    let calls = Arc::new(Mutex::new(Vec::new()));
    let mut server = WorkspaceServer::new(Workspace::seed("x"));
    server.allow_insecure_webhooks(true);
    server.set_webhook_sender(RecordingSender {
        calls: Arc::clone(&calls),
    });
    subscribe(
        &mut server,
        "http://127.0.0.1:9/hook",
        vec![WebhookEventType::CommentCreated],
    );
    server.emit_webhook(
        WebhookEventType::CommentCreated,
        "comment",
        "comment-1",
        r#"{"page_id":"p-home"}"#,
    );

    let calls = calls.lock().unwrap();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].0, "http://127.0.0.1:9/hook");
    assert!(calls[0].2.starts_with("sha256="));
}

#[test]
fn signature_roundtrip_and_tamper_rejection() {
    let secret = "secret_test";
    let body = r#"{"type":"page.created"}"#;
    let signature = sign(secret, body);
    assert!(signature.starts_with("sha256="));
    assert!(verify(secret, body, &signature));
    assert!(!verify(secret, r#"{"type":"page.deleted"}"#, &signature));
    assert!(!verify("other", body, &signature));
}

#[test]
fn page_event_delivery_has_documented_fields_and_signature() {
    let (port, received) = start_receiver(false);
    let mut server = WorkspaceServer::new(Workspace::seed("x"));
    server.allow_insecure_webhooks(true);
    let subscription = subscribe(
        &mut server,
        &format!("http://127.0.0.1:{port}/hook"),
        vec![WebhookEventType::PageCreated],
    );

    server.emit_webhook(
        WebhookEventType::PageCreated,
        "page",
        "p-home",
        r#"{"parent":{"id":"root","type":"page"}}"#,
    );
    server.flush_webhooks();

    let received = received.lock().unwrap();
    assert_eq!(received.len(), 1);
    let event: WebhookEvent = serde_json::from_str(&received[0].body).unwrap();
    assert_eq!(event.r#type, WebhookEventType::PageCreated);
    assert_eq!(event.entity.id, "p-home");
    assert_eq!(event.entity.r#type, "page");
    assert_eq!(event.subscription_id, subscription.id);
    assert!(!event.workspace_id.is_empty());
    assert!(!event.authors.is_empty());
    assert!(verify(
        &subscription.verification_token,
        &received[0].body,
        &received[0].signature
    ));
}

#[test]
fn comment_event_is_immediate() {
    let (port, received) = start_receiver(false);
    let mut server = WorkspaceServer::new(Workspace::seed("x"));
    server.allow_insecure_webhooks(true);
    subscribe(
        &mut server,
        &format!("http://127.0.0.1:{port}/hook"),
        vec![WebhookEventType::CommentCreated],
    );
    server.emit_webhook(
        WebhookEventType::CommentCreated,
        "comment",
        "comment-1",
        r#"{"page_id":"p-home"}"#,
    );
    let received = received.lock().unwrap();
    assert_eq!(received.len(), 1);
    let event: WebhookEvent = serde_json::from_str(&received[0].body).unwrap();
    assert_eq!(event.r#type, WebhookEventType::CommentCreated);
}

#[test]
fn aggregation_keeps_only_the_latest_event_per_entity() {
    let (port, received) = start_receiver(false);
    let mut server = WorkspaceServer::new(Workspace::seed("x"));
    server.allow_insecure_webhooks(true);
    subscribe(
        &mut server,
        &format!("http://127.0.0.1:{port}/hook"),
        vec![WebhookEventType::PageContentUpdated],
    );
    server.emit_webhook(
        WebhookEventType::PageContentUpdated,
        "page",
        "p-home",
        r#"{"version":1}"#,
    );
    server.emit_webhook(
        WebhookEventType::PageContentUpdated,
        "page",
        "p-home",
        r#"{"version":2}"#,
    );
    server.flush_webhooks();

    let received = received.lock().unwrap();
    assert_eq!(received.len(), 1);
    let event: WebhookEvent = serde_json::from_str(&received[0].body).unwrap();
    assert!(event.data.contains("\"version\":2"));
}

#[test]
fn failed_delivery_is_retried() {
    let (port, received) = start_receiver(true);
    let mut server = WorkspaceServer::new(Workspace::seed("x"));
    server.allow_insecure_webhooks(true);
    subscribe(
        &mut server,
        &format!("http://127.0.0.1:{port}/hook"),
        vec![WebhookEventType::CommentCreated],
    );
    server.emit_webhook(
        WebhookEventType::CommentCreated,
        "comment",
        "comment-1",
        r#"{}"#,
    );
    let first = server.webhook_deliveries().to_vec();
    assert_eq!(first.len(), 1);
    assert!(!first[0].delivered);
    assert_eq!(first[0].attempts, 1);

    server.retry_webhook_deliveries();
    let deliveries = server.webhook_deliveries();
    assert!(deliveries[0].delivered);
    assert_eq!(deliveries[0].attempts, 2);
    assert_eq!(received.lock().unwrap().len(), 2);
}

#[test]
fn v2_webhook_api_and_client_roundtrip() {
    let (port, _received) = start_receiver(false);
    let mut client = Client::with_token(WorkspaceServer::new(Workspace::seed("x")), "owner-token");
    let input = WebhookSubscriptionInput {
        url: format!("http://127.0.0.1:{port}/hook"),
        events: vec![WebhookEventType::PageCreated],
    };
    // The API rejects loopback HTTP unless the server override is enabled.
    let error = client.create_webhook_v2(&input).unwrap_err();
    assert!(matches!(error, ion_of_notes::client::ClientError::Api(_)));

    let mut server = WorkspaceServer::new(Workspace::seed("x"));
    server.allow_insecure_webhooks(true);
    let mut client = Client::with_token(server, "owner-token");
    let subscription = client.create_webhook_v2(&input).unwrap();
    client
        .verify_webhook_v2(&subscription.id, &subscription.verification_token)
        .unwrap();
    assert_eq!(client.list_webhooks_v2().unwrap().len(), 1);
    client.delete_webhook_v2(&subscription.id).unwrap();
    assert!(client.list_webhooks_v2().unwrap().is_empty());
}

#[test]
fn production_sender_returns_redirect_status_without_following_it() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            let _ = read_request(&mut stream);
            let _ = write!(
                stream,
                "HTTP/1.1 302 Found\r\nLocation: http://127.0.0.1:1/\r\n\
                 Content-Length: 0\r\nConnection: close\r\n\r\n"
            );
        }
    });
    let sender = UreqWebhookSender::new();
    let status = sender
        .send(&format!("http://127.0.0.1:{port}/hook"), "{}", "sig")
        .expect("delivery");
    assert_eq!(status, 302);
}

#[test]
fn all_documented_event_types_roundtrip() {
    let names = [
        "page.created",
        "page.content_updated",
        "page.properties_updated",
        "page.moved",
        "page.deleted",
        "page.undeleted",
        "page.locked",
        "page.unlocked",
        "page.transcription_block_transcript_deleted",
        "database.created",
        "database.content_updated",
        "database.schema_updated",
        "database.moved",
        "database.deleted",
        "database.undeleted",
        "data_source.created",
        "data_source.content_updated",
        "data_source.schema_updated",
        "data_source.moved",
        "data_source.deleted",
        "data_source.undeleted",
        "comment.created",
        "comment.updated",
        "comment.deleted",
        "file_upload.created",
        "file_upload.completed",
        "file_upload.expired",
        "file_upload.upload_failed",
        "view.created",
        "view.updated",
        "view.deleted",
    ];
    for name in names {
        let event_type = WebhookEventType::parse(name).expect(name);
        assert_eq!(event_type.as_str(), name);
    }
}

#[test]
fn aggregated_events_survive_a_restart() {
    let workspace_path = common::temp_path("webhook-queue-workspace", "json");
    let collab_path = common::temp_path("webhook-queue-collab", "json");
    common::remove(&workspace_path);
    common::remove(&collab_path);
    let workspace = Workspace::seed("webhook-queue");

    let mut server = WorkspaceServer::with_paths(
        workspace.clone(),
        workspace_path.clone(),
        collab_path.clone(),
    );
    server.allow_insecure_webhooks(true);
    subscribe(
        &mut server,
        "http://127.0.0.1:9/hook",
        vec![WebhookEventType::PageContentUpdated],
    );
    server.emit_webhook(
        WebhookEventType::PageContentUpdated,
        "page",
        "p-home",
        r#"{"version":1}"#,
    );
    assert_eq!(server.webhook_stats().pending, 1);
    drop(server);

    let mut reopened =
        WorkspaceServer::with_paths(workspace, workspace_path.clone(), collab_path.clone());
    reopened.allow_insecure_webhooks(true);
    let calls = Arc::new(Mutex::new(Vec::new()));
    reopened.set_webhook_sender(RecordingSender {
        calls: Arc::clone(&calls),
    });
    assert_eq!(reopened.webhook_stats().pending, 1);
    reopened.flush_webhooks();
    assert_eq!(reopened.webhook_stats().pending, 0);
    assert_eq!(calls.lock().unwrap().len(), 1);
    assert_eq!(reopened.webhook_deliveries().len(), 1);

    common::remove(&workspace_path);
    common::remove(&collab_path);
}

#[test]
fn failed_delivery_is_dead_lettered_and_can_be_requeued() {
    let mut server = WorkspaceServer::new(Workspace::seed("x"));
    server.allow_insecure_webhooks(true);
    server.set_webhook_sender(FailingSender);
    subscribe(
        &mut server,
        "http://127.0.0.1:9/hook",
        vec![WebhookEventType::CommentCreated],
    );
    server.emit_webhook(
        WebhookEventType::CommentCreated,
        "comment",
        "comment-1",
        r#"{}"#,
    );
    for _ in 0..8 {
        server.retry_webhook_deliveries();
    }
    let delivery = server.webhook_deliveries()[0].clone();
    assert!(delivery.dead_lettered);
    assert_eq!(server.webhook_stats().dead_letters, 1);

    let response = server.handle(&common::v2_request(
        Method::Get,
        "/api/v2/webhook_deliveries/dead_letters",
        None,
    ));
    assert_eq!(response.status, 200);
    let dead: Vec<WebhookDelivery> = common::data(&response);
    assert_eq!(dead.len(), 1);

    let body = serde_json::json!({ "id": delivery.id }).to_string();
    let response = server.handle(&common::v2_request(
        Method::Post,
        "/api/v2/webhook_deliveries/requeue",
        Some(body),
    ));
    assert_eq!(response.status, 200);
    let requeued: WebhookDelivery = common::data(&response);
    assert!(!requeued.dead_lettered);
    assert_eq!(requeued.attempts, 0);
}
