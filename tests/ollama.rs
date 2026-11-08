//! Tests for the Ollama adapter's pure helpers (public API only).
//!
//! No Ollama install or network is needed: only the request/response parsing
//! and endpoint logic are exercised here.

use not_notion::ollama::{
    DEFAULT_MODEL, DEFAULT_PORT, OllamaConfig, OllamaStatus, build_generate_body, decode_body,
    decode_chunked, default_endpoint, generate, normalize_endpoint, parse_endpoint,
    parse_generate_response, parse_tags_response,
};

#[test]
fn default_endpoint_and_config_use_documented_defaults() {
    assert_eq!(default_endpoint(), "http://127.0.0.1:11434");
    assert_eq!(DEFAULT_PORT, 11434);
    let config = OllamaConfig::default();
    assert_eq!(config.model, DEFAULT_MODEL);
    assert!(config.prefer_cli);
}

#[test]
fn endpoints_accept_host_port_and_full_urls() {
    assert_eq!(
        parse_endpoint("http://192.168.1.20:11434").unwrap(),
        ("192.168.1.20".to_string(), 11434)
    );
    assert_eq!(
        parse_endpoint("localhost:8080/api/generate").unwrap(),
        ("localhost".to_string(), 8080)
    );
    assert_eq!(
        normalize_endpoint("ollama.internal:11434"),
        "http://ollama.internal:11434"
    );
    assert_eq!(
        normalize_endpoint("https://ai.example.com/"),
        "https://ai.example.com"
    );
    assert_eq!(
        parse_endpoint("http://127.0.0.1").unwrap(),
        ("127.0.0.1".to_string(), DEFAULT_PORT)
    );
    assert!(parse_endpoint("").is_err());
    assert!(parse_endpoint("http://host:notaport").is_err());
}

#[test]
fn generate_request_body_is_non_streaming_json() {
    let body = build_generate_body("llama3.2", "Why is the sky blue?");
    let value: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(value["model"], "llama3.2");
    assert_eq!(value["prompt"], "Why is the sky blue?");
    assert_eq!(value["stream"], false);
}

#[test]
fn generate_and_tags_replies_are_parsed() {
    assert_eq!(
        parse_generate_response(r#"{"model":"m","response":"Hello there"}"#).unwrap(),
        "Hello there"
    );
    assert!(parse_generate_response(r#"{"error":"model not found"}"#).is_err());
    assert!(parse_generate_response(r#"{"model":"m"}"#).is_err());
    assert!(parse_generate_response("not json").is_err());

    let tags = r#"{"models":[{"name":"llama3.2:latest"},{"name":"qwen2.5:7b"}]}"#;
    assert_eq!(
        parse_tags_response(tags).unwrap(),
        vec!["llama3.2:latest".to_string(), "qwen2.5:7b".to_string()]
    );
    assert!(parse_tags_response("{}").is_err());
}

#[test]
fn chunked_and_plain_http_bodies_are_decoded() {
    let chunked = b"3\r\nabc\r\n4\r\ndefg\r\n0\r\n\r\n";
    assert_eq!(decode_chunked(chunked).unwrap(), b"abcdefg");

    let plain =
        b"HTTP/1.1 200 OK\r\nContent-Length: 18\r\nConnection: close\r\n\r\n{\"response\":\"ok\"}";
    assert_eq!(decode_body(plain).unwrap(), r#"{"response":"ok"}"#);

    let chunked_response = b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n5\r\nhello\r\n0\r\n\r\n";
    assert_eq!(decode_body(chunked_response).unwrap(), "hello");

    let error = b"HTTP/1.1 500 Internal Server Error\r\nContent-Length: 2\r\n\r\n{}";
    assert!(decode_body(error).is_err());
    assert!(decode_body(b"nonsense").is_err());
}

#[test]
fn status_labels_describe_both_transports() {
    assert!(OllamaStatus::Unavailable.label().contains("PATH"));
    assert!(!OllamaStatus::Unavailable.connected());
    assert!(
        OllamaStatus::Cli {
            version: "0.5.0".into()
        }
        .connected()
    );
    assert!(
        OllamaStatus::Service {
            endpoint: default_endpoint()
        }
        .connected()
    );
}

#[test]
fn empty_question_is_rejected_before_any_transport() {
    let config = OllamaConfig::default();
    assert!(generate(&config, "   ").is_err());
}
