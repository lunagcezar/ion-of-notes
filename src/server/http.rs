//! Loopback HTTP/1.1 transport adapter.

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use super::response::bad;
use super::{ApiRequest, ApiResponse, Method, RequestTransport, WorkspaceServer};

/// Bound for the request line and headers of the loopback parser.
const MAX_HEAD_BYTES: usize = 16 * 1024;
/// Bound for the number of header fields.
const MAX_HEADERS: usize = 100;

/// Parse one HTTP/1.1 request into an `ApiRequest`.
pub fn parse_http_request(bytes: &[u8]) -> Option<ApiRequest> {
    let text = String::from_utf8_lossy(bytes);
    let (head, body_bytes) = match text.split_once("\r\n\r\n") {
        Some((h, b)) => (h, b),
        None => (text.as_ref(), ""),
    };
    if head.len() > MAX_HEAD_BYTES {
        return None;
    }
    let mut lines = head.split("\r\n");
    let mut words = lines.next().unwrap_or("").split_whitespace();
    let method = Method::parse(words.next().unwrap_or(""))?;
    let path = words.next().unwrap_or("").to_string();
    if path.is_empty() || !path.starts_with('/') {
        return None;
    }
    let mut content_length: Option<usize> = None;
    let mut headers = std::collections::BTreeMap::new();
    for (count, line) in lines.enumerate() {
        if count >= MAX_HEADERS {
            return None;
        }
        if let Some((key, value)) = line.split_once(':') {
            let key = key.trim().to_ascii_lowercase();
            let value = value.trim().to_string();
            if key.is_empty() {
                return None;
            }
            if key == "content-length" {
                let length = value.parse::<usize>().ok()?;
                if content_length.is_some_and(|existing| existing != length) {
                    return None;
                }
                content_length = Some(length);
            }
            headers.insert(key, value);
        }
    }
    let content_length = content_length.unwrap_or(0);
    let raw = body_bytes.as_bytes();
    if content_length > raw.len() {
        return None;
    }
    let body =
        (content_length > 0).then(|| String::from_utf8_lossy(&raw[..content_length]).into_owned());
    Some(ApiRequest {
        method,
        path,
        body,
        headers,
        client_ip: None,
    })
}

/// Serialize an `ApiResponse` as an HTTP/1.1 response.
pub fn format_http_response(response: &ApiResponse) -> Vec<u8> {
    let reason = match response.status {
        200 => "OK",
        201 => "Created",
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        409 => "Conflict",
        429 => "Too Many Requests",
        500 => "Internal Server Error",
        503 => "Service Unavailable",
        529 => "Service Overload",
        _ => "OK",
    };
    let request_id = response
        .request_id
        .as_deref()
        .map(|id| format!("X-Request-Id: {id}\r\n"))
        .unwrap_or_default();
    let retry_after = retry_after_header(response);
    format!(
        "HTTP/1.1 {} {}\r\n{request_id}{retry_after}Content-Type: application/json; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\nCache-Control: no-store\r\n\r\n{}",
        response.status,
        reason,
        response.body.len(),
        response.body
    )
    .into_bytes()
}

/// The error body is the single source of truth: when it carries
/// `additional_data.retry_after`, the HTTP adapters emit the standard
/// `Retry-After` header (spec-037, spec-038).
pub(crate) fn retry_after_seconds(response: &ApiResponse) -> Option<u64> {
    if !matches!(response.status, 429 | 503 | 529) {
        return None;
    }
    serde_json::from_str::<serde_json::Value>(&response.body)
        .ok()
        .and_then(|body| {
            body.get("additional_data")?
                .get("retry_after")?
                .as_str()?
                .parse()
                .ok()
        })
}

fn retry_after_header(response: &ApiResponse) -> String {
    retry_after_seconds(response)
        .map(|seconds| format!("Retry-After: {seconds}\r\n"))
        .unwrap_or_default()
}

fn request_complete(buffer: &[u8]) -> bool {
    let Some(index) = buffer.windows(4).position(|w| w == b"\r\n\r\n") else {
        return false;
    };
    let head = String::from_utf8_lossy(&buffer[..index]);
    let mut length = 0usize;
    for line in head.split("\r\n").skip(1) {
        if let Some((key, value)) = line.split_once(':')
            && key.trim().eq_ignore_ascii_case("content-length")
        {
            length = value.trim().parse().unwrap_or(0);
        }
    }
    buffer.len() >= index + 4 + length
}

/// Read one HTTP request from a connection. `None` means the bytes were not a
/// valid request (malformed head, bad content length, truncated body).
fn read_request(stream: &mut TcpStream) -> std::io::Result<Option<ApiRequest>> {
    stream.set_read_timeout(Some(Duration::from_secs(2)))?;
    let mut buffer = Vec::new();
    let mut chunk = [0u8; 4096];
    for _ in 0..32 {
        if request_complete(&buffer) {
            break;
        }
        match stream.read(&mut chunk) {
            Ok(0) => break,
            Ok(n) => buffer.extend_from_slice(&chunk[..n]),
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
            Err(e) if e.kind() == std::io::ErrorKind::TimedOut => break,
            Err(e) => return Err(e),
        }
    }
    Ok(parse_http_request(&buffer))
}

/// Handle one connection and write one response.
pub fn serve_once<A: RequestTransport>(api: &mut A, stream: &mut TcpStream) -> std::io::Result<()> {
    let response = match read_request(stream)? {
        Some(request) => api.handle(&request),
        None => bad("malformed HTTP request"),
    };
    stream.write_all(&format_http_response(&response))?;
    stream.flush()
}

/// Serve connections from an already-bound listener.
///
/// The listener is handed to the Axum adapter, so the loopback and `--serve`
/// paths share one HTTP implementation. `serve_once` remains a lower-level
/// helper for tests of the parser and response formatter.
pub fn serve_on(listener: TcpListener, server: WorkspaceServer) -> std::io::Result<()> {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;
    runtime.block_on(async move {
        listener.set_nonblocking(true)?;
        let listener = tokio::net::TcpListener::from_std(listener)?;
        let server = Arc::new(Mutex::new(server));
        axum::serve(listener, super::web::router_with_server(server)).await
    })
}

/// Blocking loopback API server. Only used by `--serve`.
pub fn serve_forever(addr: &str, server: WorkspaceServer) -> std::io::Result<()> {
    let listener = TcpListener::bind(addr)?;
    println!("ion-of-notes API listening on http://{addr}");
    serve_on(listener, server)
}
