//! AI assistant integration with [Ollama](https://ollama.com).
//!
//! The person asked for the assistant to work with **either** the `ollama`
//! command-line tool in `PATH` **or** a service reachable at a network port
//! (`host:port`). This module detects both, builds the OpenAI-independent
//! `/api/generate` request with the standard library only (no HTTP crate), and
//! turns the reply into plain text. Network and process calls run on a
//! background thread from `ui::app`, so the window never freezes.
//!
//! The pure helpers (`build_generate_body`, `parse_generate_response`,
//! `parse_endpoint`, `decode_body`, `parse_tags_response`) are testable without
//! a running Ollama, which keeps this adapter behind the same seam as the
//! workspace server.

use serde_json::{json, Value};
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
use std::process::Command;
use std::time::Duration;

/// Default loopback endpoint of the Ollama service.
pub const DEFAULT_HOST: &str = "127.0.0.1";
pub const DEFAULT_PORT: u16 = 11434;
/// Model used when none is configured. Any installed Ollama model works.
pub const DEFAULT_MODEL: &str = "llama3.2";

pub fn default_endpoint() -> String {
    format!("http://{DEFAULT_HOST}:{DEFAULT_PORT}")
}

/// How the assistant reaches Ollama.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OllamaConfig {
    /// `http://host:port` (scheme and port are optional).
    pub endpoint: String,
    pub model: String,
    /// Prefer the local CLI when it is installed; otherwise use the service.
    pub prefer_cli: bool,
}

impl Default for OllamaConfig {
    fn default() -> Self {
        Self {
            endpoint: default_endpoint(),
            model: DEFAULT_MODEL.to_string(),
            prefer_cli: true,
        }
    }
}

impl OllamaConfig {
    /// Build a config from the environment: `OLLAMA_HOST` (a host, a
    /// `host:port` or a full URL) and `NOT_NOTION_OLLAMA_MODEL`. When no model
    /// is configured, the first model reported by the service is used, so the
    /// assistant works without any setup on a machine that already runs Ollama.
    pub fn from_env() -> Self {
        let endpoint = std::env::var("OLLAMA_HOST")
            .ok()
            .filter(|v| !v.trim().is_empty())
            .map(|v| normalize_endpoint(&v))
            .unwrap_or_else(default_endpoint);
        let model = std::env::var("NOT_NOTION_OLLAMA_MODEL")
            .ok()
            .filter(|v| !v.trim().is_empty())
            .unwrap_or_else(|| {
                default_model_for(&endpoint).unwrap_or_else(|| DEFAULT_MODEL.to_string())
            });
        Self { endpoint, model, prefer_cli: true }
    }
}

/// Add a scheme when the user supplied only a host or `host:port`.
pub fn normalize_endpoint(value: &str) -> String {
    let v = value.trim().trim_end_matches('/');
    if v.starts_with("http://") || v.starts_with("https://") {
        v.to_string()
    } else {
        format!("http://{v}")
    }
}

/// Split an endpoint into `(host, port)`. A missing port defaults to 11434.
pub fn parse_endpoint(endpoint: &str) -> Result<(String, u16), String> {
    let mut s = endpoint.trim();
    for prefix in ["http://", "https://"] {
        if let Some(rest) = s.strip_prefix(prefix) {
            s = rest;
            break;
        }
    }
    let authority = s
        .split(['/', '?', '#'])
        .next()
        .unwrap_or("");
    if authority.is_empty() {
        return Err("missing host in Ollama endpoint".into());
    }
    let (host, port) = match authority.rsplit_once(':') {
        Some((h, p)) => {
            let port: u16 = p
                .parse()
                .map_err(|_| format!("invalid Ollama port '{p}'"))?;
            (h.to_string(), port)
        }
        None => (authority.to_string(), DEFAULT_PORT),
    };
    if host.is_empty() || port == 0 {
        return Err("missing host or invalid port in Ollama endpoint".into());
    }
    Ok((host, port))
}

fn socket_addr(endpoint: &str) -> Result<SocketAddr, String> {
    let (host, port) = parse_endpoint(endpoint)?;
    (host.as_str(), port)
        .to_socket_addrs()
        .map_err(|e| format!("cannot resolve {host}:{port}: {e}"))?
        .next()
        .ok_or_else(|| format!("no address for {host}:{port}"))
}

/// Status of the local/network Ollama installation, shown in Settings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OllamaStatus {
    Cli { version: String },
    Service { endpoint: String },
    Unavailable,
}

impl OllamaStatus {
    pub fn connected(&self) -> bool {
        !matches!(self, OllamaStatus::Unavailable)
    }

    pub fn label(&self) -> String {
        match self {
            OllamaStatus::Cli { version } => {
                format!("Ollama CLI in PATH · {version}")
            }
            OllamaStatus::Service { endpoint } => {
                format!("Ollama service · {endpoint}")
            }
            OllamaStatus::Unavailable => {
                "Not connected · put the ollama CLI in PATH or start the service on a reachable port".to_string()
            }
        }
    }
}

/// Ask `ollama --version` to see whether the CLI is installed.
pub fn cli_version() -> Option<String> {
    let out = Command::new("ollama").arg("--version").output().ok()?;
    if !out.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if text.is_empty() { None } else { Some(text) }
}

/// A quick TCP probe of the configured service (short timeout).
pub fn service_reachable(endpoint: &str) -> bool {
    socket_addr(endpoint)
        .ok()
        .map(|addr| TcpStream::connect_timeout(&addr, Duration::from_millis(250)).is_ok())
        .unwrap_or(false)
}

/// Detect the best available transport, honouring `prefer_cli`.
pub fn detect(config: &OllamaConfig) -> OllamaStatus {
    let cli = cli_version();
    if config.prefer_cli
        && let Some(version) = &cli
    {
        return OllamaStatus::Cli { version: version.clone() };
    }
    if service_reachable(&config.endpoint) {
        return OllamaStatus::Service { endpoint: config.endpoint.clone() };
    }
    match cli {
        Some(version) => OllamaStatus::Cli { version },
        None => OllamaStatus::Unavailable,
    }
}

/// Body of a non-streaming `/api/generate` request.
pub fn build_generate_body(model: &str, prompt: &str) -> String {
    json!({ "model": model, "prompt": prompt, "stream": false }).to_string()
}

/// Extract the generated text from a `/api/generate` JSON reply.
pub fn parse_generate_response(body: &str) -> Result<String, String> {
    let value: Value = serde_json::from_str(body)
        .map_err(|e| format!("invalid Ollama reply: {e}"))?;
    if let Some(err) = value.get("error").and_then(Value::as_str) {
        return Err(err.to_string());
    }
    value
        .get("response")
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| "Ollama reply has no 'response' field".to_string())
}

/// Model names from a `/api/tags` reply.
pub fn parse_tags_response(body: &str) -> Result<Vec<String>, String> {
    let value: Value = serde_json::from_str(body)
        .map_err(|e| format!("invalid Ollama tags reply: {e}"))?;
    let models = value
        .get("models")
        .and_then(Value::as_array)
        .ok_or_else(|| "Ollama tags reply has no 'models' array".to_string())?;
    Ok(models
        .iter()
        .filter_map(|m| m.get("name").and_then(Value::as_str).map(str::to_string))
        .collect())
}

fn find_crlf(bytes: &[u8]) -> Option<usize> {
    bytes.windows(2).position(|w| w == b"\r\n")
}

/// Decode a simple HTTP/1.1 `Transfer-Encoding: chunked` body.
pub fn decode_chunked(bytes: &[u8]) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    let mut i = 0usize;
    loop {
        let nl = find_crlf(&bytes[i..]).ok_or("truncated chunk header")?;
        let line = std::str::from_utf8(&bytes[i..i + nl])
            .map_err(|e| format!("bad chunk size: {e}"))?;
        let size = usize::from_str_radix(line.split(';').next().unwrap_or("").trim(), 16)
            .map_err(|e| format!("bad chunk size: {e}"))?;
        i += nl + 2;
        if size == 0 {
            break;
        }
        if i + size > bytes.len() {
            return Err("truncated chunk data".into());
        }
        out.extend_from_slice(&bytes[i..i + size]);
        i += size + 2; // skip the CRLF after the chunk
    }
    Ok(out)
}

/// Split a raw HTTP response into its JSON body, checking the status line.
pub fn decode_body(raw: &[u8]) -> Result<String, String> {
    let split = raw
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .map(|p| p + 4)
        .ok_or_else(|| "malformed HTTP response (no header/body split)".to_string())?;
    let head = String::from_utf8_lossy(&raw[..split]).to_string();
    let status = head.lines().next().unwrap_or("").trim().to_string();
    if !status.contains(" 200 ") && !status.ends_with(" 200") {
        return Err(format!("Ollama returned {status}"));
    }
    let chunked = head.to_ascii_lowercase().contains("transfer-encoding: chunked");
    let body = if chunked {
        String::from_utf8_lossy(&decode_chunked(&raw[split..])?).into_owned()
    } else {
        String::from_utf8_lossy(&raw[split..]).into_owned()
    };
    Ok(body)
}

/// One small blocking HTTP request against the Ollama service.
fn http_request(
    endpoint: &str,
    method: &str,
    path: &str,
    body: Option<&str>,
) -> Result<String, String> {
    let (host, port) = parse_endpoint(endpoint)?;
    let addr = socket_addr(endpoint)?;
    let mut stream = TcpStream::connect_timeout(&addr, Duration::from_secs(3))
        .map_err(|e| format!("cannot reach {host}:{port}: {e}"))?;
    stream
        .set_read_timeout(Some(Duration::from_secs(120)))
        .map_err(|e| e.to_string())?;
    let payload = body.unwrap_or("");
    let request = format!(
        "{method} {path} HTTP/1.1\r\nHost: {host}:{port}\r\n\
         Accept: application/json\r\nContent-Type: application/json\r\n\
         Content-Length: {}\r\nConnection: close\r\n\r\n{payload}",
        payload.len()
    );
    stream
        .write_all(request.as_bytes())
        .map_err(|e| format!("cannot send request: {e}"))?;
    let mut raw = Vec::new();
    stream
        .read_to_end(&mut raw)
        .map_err(|e| format!("cannot read reply: {e}"))?;
    decode_body(&raw)
}

/// Models installed on the reachable Ollama service.
pub fn models(config: &OllamaConfig) -> Result<Vec<String>, String> {
    let raw = http_request(&config.endpoint, "GET", "/api/tags", None)?;
    parse_tags_response(&raw)
}

/// The first installed model on the service, or `None` when it is unreachable.
/// The short reachability probe keeps startup fast when Ollama is not running.
pub fn default_model_for(endpoint: &str) -> Option<String> {
    if !service_reachable(endpoint) {
        return None;
    }
    let config = OllamaConfig {
        endpoint: endpoint.to_string(),
        model: String::new(),
        prefer_cli: false,
    };
    models(&config).ok()?.into_iter().next()
}

fn generate_cli(config: &OllamaConfig, prompt: &str) -> Result<String, String> {
    let out = Command::new("ollama")
        .args(["run", &config.model, prompt])
        .output()
        .map_err(|e| format!("cannot run ollama: {e}"))?;
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr).trim().to_string();
        return Err(if err.is_empty() {
            "ollama run failed".to_string()
        } else {
            err
        });
    }
    let text = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if text.is_empty() {
        Err("ollama returned an empty answer".to_string())
    } else {
        Ok(text)
    }
}

fn generate_http(config: &OllamaConfig, prompt: &str) -> Result<String, String> {
    let body = build_generate_body(&config.model, prompt);
    let raw = http_request(&config.endpoint, "POST", "/api/generate", Some(&body))?;
    parse_generate_response(&raw)
}

/// Generate an answer for `prompt`, using whichever transport is available.
pub fn generate(config: &OllamaConfig, prompt: &str) -> Result<String, String> {
    if prompt.trim().is_empty() {
        return Err("Ask a question first.".to_string());
    }
    match detect(config) {
        OllamaStatus::Cli { .. } => generate_cli(config, prompt),
        OllamaStatus::Service { .. } => generate_http(config, prompt),
        OllamaStatus::Unavailable => Err(
            "Ollama is not available. Install the `ollama` CLI in PATH, or start the \
             service and set OLLAMA_HOST to a reachable host:port."
                .to_string(),
        ),
    }
}
