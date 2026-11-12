use super::*;

pub(super) fn agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(15)))
        .http_status_as_error(false)
        .max_redirects(2)
        .build()
        .into()
}

pub(super) fn get_json<T: DeserializeOwned>(
    agent: &ureq::Agent,
    url: &str,
    token: Option<&str>,
) -> Result<T, String> {
    let mut request = agent.get(url);
    if let Some(token) = token {
        request = request.header(AUTH_HEADER, format!("Bearer {token}"));
    }
    let mut response = request.call().map_err(|error| error.to_string())?;
    let status = response.status().as_u16();
    let text = response
        .body_mut()
        .read_to_string()
        .map_err(|error| error.to_string())?;
    if !(200..300).contains(&status) {
        return Err(text);
    }
    let envelope: Envelope<T> = serde_json::from_str(&text).map_err(|error| error.to_string())?;
    envelope.data.ok_or_else(|| {
        envelope
            .error
            .unwrap_or_else(|| "empty response".to_string())
    })
}

pub(super) fn execute(
    agent: &ureq::Agent,
    method: Method,
    url: &str,
    headers: &BTreeMap<String, String>,
    body: Option<&str>,
) -> Result<(u16, Option<String>, String), String> {
    let mut response = match method {
        Method::Get => {
            let mut builder = agent.get(url);
            for (name, value) in headers {
                builder = builder.header(name, value);
            }
            builder.call()
        }
        Method::Delete => {
            let mut builder = agent.delete(url);
            for (name, value) in headers {
                builder = builder.header(name, value);
            }
            builder.call()
        }
        Method::Post => {
            let mut builder = agent.post(url);
            for (name, value) in headers {
                builder = builder.header(name, value);
            }
            builder.send(body.unwrap_or(""))
        }
        Method::Put => {
            let mut builder = agent.put(url);
            for (name, value) in headers {
                builder = builder.header(name, value);
            }
            builder.send(body.unwrap_or(""))
        }
        Method::Patch => {
            let mut builder = agent.patch(url);
            for (name, value) in headers {
                builder = builder.header(name, value);
            }
            builder.send(body.unwrap_or(""))
        }
    }
    .map_err(|error| error.to_string())?;
    let status = response.status().as_u16();
    let request_id = response
        .headers()
        .get("x-request-id")
        .and_then(|value| value.to_str().ok())
        .map(str::to_string);
    let body = response
        .body_mut()
        .read_to_string()
        .map_err(|error| error.to_string())?;
    Ok((status, request_id, body))
}
