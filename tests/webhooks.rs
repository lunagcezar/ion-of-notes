//! Webhook subscription, signature, aggregation and delivery tests (spec-036).

mod common;

use ion_of_notes::client::Client;
use ion_of_notes::model::Workspace;
use ion_of_notes::server::WorkspaceServer;
use ion_of_notes::webhooks::{
    SubscriptionStatus, WebhookEvent, WebhookEventType, WebhookSubscription,
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

    server.process_webhook_deliveries();
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
