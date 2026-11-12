use super::*;

pub(super) async fn handle(State(state): State<AppState>, request: Request) -> Response {
    state.requests.fetch_add(1, Ordering::Relaxed);
    let (parts, body) = request.into_parts();
    let body = match axum::body::to_bytes(body, MAX_BODY_BYTES).await {
        Ok(bytes) => (!bytes.is_empty()).then(|| String::from_utf8_lossy(&bytes).into_owned()),
        Err(_) => {
            return plain_error(StatusCode::PAYLOAD_TOO_LARGE, "request body is too large");
        }
    };
    let Some(method) = api_method(parts.method.as_str()) else {
        return plain_error(StatusCode::METHOD_NOT_ALLOWED, "unsupported method");
    };
    let path = parts
        .uri
        .path_and_query()
        .map(|path| path.as_str())
        .unwrap_or("/")
        .to_string();
    let headers: BTreeMap<String, String> = parts
        .headers
        .iter()
        .filter_map(|(name, value)| {
            value
                .to_str()
                .ok()
                .map(|value| (name.as_str().to_string(), value.to_string()))
        })
        .collect();
    let client_ip = headers
        .get("x-forwarded-for")
        .and_then(|value| value.split(',').next())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .or_else(|| headers.get("x-real-ip").cloned());
    let api_request = ApiRequest {
        method,
        path,
        body,
        headers,
        client_ip,
    };

    let server = Arc::clone(&state.server);
    let response = match tokio::task::spawn_blocking(move || {
        let mut guard = server
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        guard.handle(&api_request)
    })
    .await
    {
        Ok(response) => response,
        Err(error) => {
            return plain_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("server task failed: {error}"),
            );
        }
    };
    to_http(response)
}

fn api_method(method: &str) -> Option<Method> {
    match method {
        "GET" => Some(Method::Get),
        "POST" => Some(Method::Post),
        "PUT" => Some(Method::Put),
        "PATCH" => Some(Method::Patch),
        "DELETE" => Some(Method::Delete),
        _ => None,
    }
}

fn to_http(response: ApiResponse) -> Response {
    let status = StatusCode::from_u16(response.status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
    let mut builder = Response::builder()
        .status(status)
        .header(header::CONTENT_TYPE, "application/json; charset=utf-8");
    if let Some(request_id) = &response.request_id {
        builder = builder.header("x-request-id", request_id);
    }
    if let Some(seconds) = retry_after_seconds(&response) {
        builder = builder.header(header::RETRY_AFTER, seconds.to_string());
    }
    builder.body(Body::from(response.body)).unwrap_or_else(|_| {
        plain_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "could not build the response",
        )
    })
}

pub(super) fn plain_error(status: StatusCode, message: &str) -> Response {
    (status, message.to_string()).into_response()
}
