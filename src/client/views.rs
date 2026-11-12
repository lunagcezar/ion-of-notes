use super::*;

impl<A: RequestTransport> Client<A> {
    pub fn list_views_v2(&mut self, data_source: Option<&str>) -> Result<Vec<View>, ClientError> {
        let path = match data_source {
            Some(id) => format!("/api/v2/views?data_source={}", url_encode(id)),
            None => "/api/v2/views".to_string(),
        };
        let request = self.v2_request(Method::Get, path, None)?;
        self.call_v2(request)
    }

    pub fn create_view_v2(&mut self, input: &ViewInput) -> Result<View, ClientError> {
        let body = encode_body(input)?;
        let request = self.v2_request(Method::Post, "/api/v2/views", Some(body))?;
        self.call_v2(request)
    }

    pub fn get_view_v2(&mut self, id: &str) -> Result<View, ClientError> {
        let request = self.v2_request(Method::Get, format!("/api/v2/views/{id}"), None)?;
        self.call_v2(request)
    }

    pub fn update_view_v2(&mut self, id: &str, input: &ViewInput) -> Result<View, ClientError> {
        let body = encode_body(input)?;
        let request = self.v2_request(Method::Patch, format!("/api/v2/views/{id}"), Some(body))?;
        self.call_v2(request)
    }

    pub fn delete_view_v2(&mut self, id: &str) -> Result<String, ClientError> {
        let request = self.v2_request(Method::Delete, format!("/api/v2/views/{id}"), None)?;
        self.call_v2(request)
    }

    pub fn query_view_v2(
        &mut self,
        id: &str,
        input: Option<&ViewQueryInput>,
        page_size: u16,
        cursor: Option<&str>,
    ) -> Result<ViewQueryResult, ClientError> {
        let body = match input {
            Some(input) => Some(encode_body(input)?),
            None => None,
        };
        let path = Self::paged_path(&format!("/api/v2/views/{id}/query"), page_size, cursor);
        let request = self.v2_request(Method::Post, path, body)?;
        self.call_v2(request)
    }
}
