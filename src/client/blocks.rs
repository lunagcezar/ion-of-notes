use super::*;

impl<A: RequestTransport> Client<A> {
    pub fn get_block_v2(&mut self, id: &str) -> Result<BlockDto, ClientError> {
        let request = self.v2_request(Method::Get, format!("/api/v2/blocks/{id}"), None)?;
        self.call_v2(request)
    }

    pub fn list_block_children_page_v2(
        &mut self,
        id: &str,
        page_size: u16,
        cursor: Option<&str>,
    ) -> Result<ListResponse<BlockDto>, ClientError> {
        let path = format!("/api/v2/blocks/{id}/children");
        self.list_page_v2(&path, page_size, cursor)
    }

    pub fn list_block_children_v2(&mut self, id: &str) -> Result<Vec<BlockDto>, ClientError> {
        let path = format!("/api/v2/blocks/{id}/children");
        self.collect_all_v2(&path, 100)
    }

    pub fn append_block_children_v2(
        &mut self,
        id: &str,
        children: &[BlockInput],
    ) -> Result<Vec<BlockDto>, ClientError> {
        let body = encode_body(&BlockChildrenRequest {
            children: children.to_vec(),
        })?;
        let request = self.v2_request(
            Method::Patch,
            format!("/api/v2/blocks/{id}/children"),
            Some(body),
        )?;
        self.call_v2(request)
    }

    pub fn update_block_v2(
        &mut self,
        id: &str,
        input: &BlockInput,
    ) -> Result<BlockDto, ClientError> {
        let body = encode_body(input)?;
        let request = self.v2_request(Method::Patch, format!("/api/v2/blocks/{id}"), Some(body))?;
        self.call_v2(request)
    }

    pub fn delete_block_v2(&mut self, id: &str) -> Result<BlockDto, ClientError> {
        let request = self.v2_request(Method::Delete, format!("/api/v2/blocks/{id}"), None)?;
        self.call_v2(request)
    }
}
