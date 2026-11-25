//! Minimal browser UI served by the Axum adapter (spec-153).
//!
//! The assets are embedded at build time. There is no bundler and no external
//! origin: the page talks to the same `/api/v2` contract as the desktop app.

use axum::http::{HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};

const HTML: &str = include_str!("assets/app.html");
const JS: &str = include_str!("assets/app.js");
const CSS: &str = include_str!("assets/app.css");

const CSP: &str = "default-src 'self'; connect-src 'self'; img-src 'self' data:; \
                   style-src 'self'; script-src 'self'; frame-ancestors 'none'";

pub(super) async fn app_html() -> Response {
    asset("text/html; charset=utf-8", HTML)
}

pub(super) async fn app_js() -> Response {
    asset("text/javascript; charset=utf-8", JS)
}

pub(super) async fn app_css() -> Response {
    asset("text/css; charset=utf-8", CSS)
}

fn asset(content_type: &'static str, body: &'static str) -> Response {
    let mut response = (StatusCode::OK, body).into_response();
    let headers = response.headers_mut();
    headers.insert(header::CONTENT_TYPE, HeaderValue::from_static(content_type));
    headers.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("no-store, max-age=0"),
    );
    headers.insert(
        header::CONTENT_SECURITY_POLICY,
        HeaderValue::from_static(CSP),
    );
    headers.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    headers.insert(
        header::REFERRER_POLICY,
        HeaderValue::from_static("no-referrer"),
    );
    headers.insert(header::X_FRAME_OPTIONS, HeaderValue::from_static("DENY"));
    response
}
