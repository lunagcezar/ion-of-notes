//! Loopback HTTP/1.1 transport adapter.

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use super::{ApiRequest, ApiResponse, Method, WorkspaceApi, WorkspaceServer};

/// Parse one HTTP/1.1 request into an `ApiRequest`.
pub fn parse_http_request(bytes: &[u8]) -> Option<ApiRequest> {
    let text = String::from_utf8_lossy(bytes);
    let (head, body_bytes) = match text.split_once("\r\n\r\n") {
        Some((h, b)) => (h, b),
        None => (text.as_ref(), ""),
    };
    let mut lines = head.split("\r\n");
    let mut words = lines.next().unwrap_or("").split_whitespace();
    let method = Method::parse(words.next().unwrap_or(""))?;
    let path = words.next().unwrap_or("").to_string();
    if path.is_empty() {
        return None;
    }
    let mut content_length = 0usize;
    let mut headers = std::collections::BTreeMap::new();
    for line in lines {
        if let Some((key, value)) = line.split_once(':') {
            let key = key.trim().to_ascii_lowercase();
            let value = value.trim().to_string();
            if key == "content-length" {
                content_length = value.parse().unwrap_or(0);
            }
            headers.insert(key, value);
        }
    }
    let body = if content_length > 0 {
        let raw = body_bytes.as_bytes();
        let end = content_length.min(raw.len());
        Some(String::from_utf8_lossy(&raw[..end]).into_owned())
    } else {
        None
    };
    Some(ApiRequest {
        method,
        path,
        body,
        headers,
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
        500 => "Internal Server Error",
        _ => "OK",
    };
    let request_id = response
        .request_id
        .as_deref()
        .map(|id| format!("X-Request-Id: {id}\r\n"))
        .unwrap_or_default();
    format!(
        "HTTP/1.1 {} {}\r\n{request_id}Content-Type: application/json; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\nCache-Control: no-store\r\n\r\n{}",
        response.status,
        reason,
        response.body.len(),
        response.body
    )
    .into_bytes()
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

/// Read one HTTP request from a connection.
fn read_request(stream: &mut TcpStream) -> std::io::Result<ApiRequest> {
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
    Ok(
        parse_http_request(&buffer)
            .unwrap_or_else(|| ApiRequest::new(Method::Get, "/health", None)),
    )
}

/// Handle one connection and write one response.
pub fn serve_once<A: WorkspaceApi>(api: &mut A, stream: &mut TcpStream) -> std::io::Result<()> {
    let request = read_request(stream)?;
    let response = api.handle(&request);
    stream.write_all(&format_http_response(&response))?;
    stream.flush()
}

/// Serve connections from an already-bound listener.
///
/// Requests are read on the connection thread; the server state is locked only
/// for `handle`, so a slow client cannot hold the state while it is still
/// sending its request (spec-021).
pub fn serve_on(listener: TcpListener, server: WorkspaceServer) -> std::io::Result<()> {
    let server = Arc::new(Mutex::new(server));
    for incoming in listener.incoming() {
        match incoming {
            Ok(mut stream) => {
                let server = Arc::clone(&server);
                std::thread::spawn(move || {
                    let request = match read_request(&mut stream) {
                        Ok(request) => request,
                        Err(error) => {
                            eprintln!("request error: {error}");
                            return;
                        }
                    };
                    let response = match server.lock() {
                        Ok(mut guard) => guard.handle(&request),
                        Err(error) => {
                            eprintln!("server lock poisoned: {error}");
                            return;
                        }
                    };
                    if let Err(error) = stream
                        .write_all(&format_http_response(&response))
                        .and_then(|()| stream.flush())
                    {
                        eprintln!("response error: {error}");
                    }
                });
            }
            Err(error) => eprintln!("connection error: {error}"),
        }
    }
    Ok(())
}

/// Blocking loopback API server. Only used by `--serve`.
pub fn serve_forever(addr: &str, server: WorkspaceServer) -> std::io::Result<()> {
    let listener = TcpListener::bind(addr)?;
    println!("not-notion API listening on http://{addr}");
    serve_on(listener, server)
}
