//! v2 request pipeline: version, auth, limits, capability, then the shared
//! resource handlers.

use super::super::contract::{CODE_NOT_FOUND, RequestLogEntry, new_request_id};
use super::super::error::handler_error;
use super::super::response::{v2_error, write_timeout_response};
use super::super::{ApiRequest, ApiResponse, Method, WorkspaceServer};
use super::pipeline;

impl WorkspaceServer {
    /// Dispatch a v2 request: version, auth, capability, then the shared v1
    /// use case. The v2 layer changes the envelope/error shape and adds a
    /// request id; it does not duplicate the business logic.
    pub(super) fn handle_v2(&mut self, request: &ApiRequest) -> ApiResponse {
        let request_id = new_request_id();
        let (path, query) = match request.path.split_once('?') {
            Some((p, q)) => (p, q),
            None => (request.path.as_str(), ""),
        };
        let is_write = !matches!(request.method, Method::Get);
        let timed_out = is_write && self.runtime.simulate_write_timeout;
        if timed_out {
            self.runtime.simulate_write_timeout = false;
        }
        let response = self
            .handle_v2_inner(request, path, query, &request_id)
            .unwrap_or_else(|error| error);
        let response = if timed_out && (200..300).contains(&response.status) {
            write_timeout_response(&response, &request_id)
        } else {
            response
        };
        let status = response.status;
        let response = ApiResponse {
            request_id: Some(request_id.clone()),
            ..response
        };
        self.runtime.request_log.push(RequestLogEntry {
            request_id,
            method: request.method.as_str().to_string(),
            path: request.path.clone(),
            status,
        });
        if self.runtime.request_log.len() > self.runtime.request_log_cap {
            let excess = self.runtime.request_log.len() - self.runtime.request_log_cap;
            self.runtime.request_log.drain(0..excess);
        }
        response
    }

    pub(super) fn handle_v2_inner(
        &mut self,
        request: &ApiRequest,
        path: &str,
        query: &str,
        request_id: &str,
    ) -> Result<ApiResponse, ApiResponse> {
        pipeline::check_version(request, request_id)?;

        if request.method == Method::Post && path == "/api/v2/auth/login" {
            pipeline::check_login_rate(self, request, request_id)?;
            return Ok(self.login(request, request_id));
        }

        let connection = pipeline::authenticate(self, request, request_id)?;
        pipeline::check_runtime(self, &connection, request, path, request_id)?;
        pipeline::check_capability(&connection, request, path, request_id)?;
        pipeline::check_authorization(self, &connection, request, path, request_id)?;

        if let Some(response) = self.handle_v2_crdt(request, path, request_id, &connection) {
            return Ok(response);
        }
        if let Some(response) = self.handle_v2_auth(request, path, request_id, &connection) {
            return Ok(response);
        }

        self.workspace.ensure_data_sources();
        self.workspace.ensure_views();
        if let Some(response) = self.handle_v2_webhook(request, path, request_id) {
            return Ok(response);
        }
        if let Some(response) = self
            .handle_v2_comments(request, path, query, request_id)
            .map_err(|error| handler_error(error, request_id))?
        {
            return Ok(response);
        }
        if let Some(response) =
            self.handle_v2_user_file(request, path, query, request_id, &connection)
        {
            return Ok(response);
        }
        if let Some(response) = self.handle_v2_view(request, path, query, request_id) {
            return Ok(response);
        }
        if let Some(response) = self.handle_v2_database(request, path, query, request_id) {
            return Ok(response);
        }
        if let Some(response) = self.handle_v2_block(request, path, query, request_id) {
            return Ok(response);
        }
        if let Some(response) = self.handle_v2_shared(request, path, request_id) {
            return Ok(response);
        }
        if let Some(response) = self.handle_v2_list(request, path, query, request_id) {
            return Ok(response);
        }

        Err(v2_error(
            404,
            CODE_NOT_FOUND,
            "unknown route",
            request_id,
            None,
        ))
    }
}
