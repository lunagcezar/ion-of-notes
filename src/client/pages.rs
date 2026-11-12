use super::*;

impl<A: RequestTransport> Client<A> {
    pub fn workspace(&mut self) -> Result<Workspace, ClientError> {
        self.call(ApiRequest::new(Method::Get, "/api/workspace", None))
    }

    pub fn replace_workspace(&mut self, workspace: &Workspace) -> Result<Workspace, ClientError> {
        let body = encode_body(workspace)?;
        self.call(ApiRequest::new(Method::Put, "/api/workspace", Some(body)))
    }

    pub fn list_pages(&mut self) -> Result<Vec<PageSummary>, ClientError> {
        self.call(ApiRequest::new(Method::Get, "/api/pages", None))
    }

    pub fn get_page(&mut self, id: &str) -> Result<Page, ClientError> {
        self.call(ApiRequest::new(
            Method::Get,
            format!("/api/pages/{id}"),
            None,
        ))
    }

    pub fn create_page(&mut self, draft: &PageDraft) -> Result<Page, ClientError> {
        let body = encode_body(draft)?;
        self.call(ApiRequest::new(Method::Post, "/api/pages", Some(body)))
    }

    pub fn update_page(&mut self, page: &Page) -> Result<Page, ClientError> {
        let body = encode_body(page)?;
        self.call(ApiRequest::new(
            Method::Patch,
            format!("/api/pages/{}", page.id),
            Some(body),
        ))
    }

    pub fn delete_page(&mut self, id: &str) -> Result<Vec<String>, ClientError> {
        self.call(ApiRequest::new(
            Method::Delete,
            format!("/api/pages/{id}"),
            None,
        ))
    }

    /// Newest-first snapshots recorded by the server (version history).
    pub fn page_versions(&mut self, id: &str) -> Result<Vec<PageVersion>, ClientError> {
        self.call(ApiRequest::new(
            Method::Get,
            format!("/api/pages/{id}/versions"),
            None,
        ))
    }

    pub fn search(&mut self, query: &str) -> Result<Vec<SearchResult>, ClientError> {
        let encoded = url_encode(query);
        self.call(ApiRequest::new(
            Method::Get,
            format!("/api/search?q={encoded}"),
            None,
        ))
    }

    // ---- decided collaborative API ---------------------------------------
    pub fn workspace_v2(&mut self) -> Result<Workspace, ClientError> {
        let request = self.v2_request(Method::Get, "/api/v2/workspace", None)?;
        self.call_v2(request)
    }

    pub fn replace_workspace_v2(
        &mut self,
        workspace: &Workspace,
    ) -> Result<Workspace, ClientError> {
        let body = encode_body(workspace)?;
        let request = self.v2_request(Method::Put, "/api/v2/workspace", Some(body))?;
        self.call_v2(request)
    }

    pub fn list_pages_v2(&mut self) -> Result<Vec<PageSummary>, ClientError> {
        self.collect_all_v2("/api/v2/pages", 100)
    }

    pub fn list_pages_page_v2(
        &mut self,
        page_size: u16,
        cursor: Option<&str>,
    ) -> Result<ListResponse<PageSummary>, ClientError> {
        self.list_page_v2("/api/v2/pages", page_size, cursor)
    }

    pub fn get_page_v2(&mut self, id: &str) -> Result<Page, ClientError> {
        let request = self.v2_request(Method::Get, format!("/api/v2/pages/{id}"), None)?;
        self.call_v2(request)
    }

    pub fn create_page_v2(&mut self, draft: &PageDraft) -> Result<Page, ClientError> {
        let body = encode_body(draft)?;
        let request = self.v2_request(Method::Post, "/api/v2/pages", Some(body))?;
        self.call_v2(request)
    }

    pub fn update_page_v2(&mut self, page: &Page) -> Result<Page, ClientError> {
        let body = encode_body(page)?;
        let request = self.v2_request(
            Method::Patch,
            format!("/api/v2/pages/{}", page.id),
            Some(body),
        )?;
        self.call_v2(request)
    }

    pub fn delete_page_v2(&mut self, id: &str) -> Result<Vec<String>, ClientError> {
        let request = self.v2_request(Method::Delete, format!("/api/v2/pages/{id}"), None)?;
        self.call_v2(request)
    }

    pub fn page_versions_v2(&mut self, id: &str) -> Result<Vec<PageVersion>, ClientError> {
        let path = format!("/api/v2/pages/{id}/versions");
        self.collect_all_v2(&path, 100)
    }

    pub fn page_versions_page_v2(
        &mut self,
        id: &str,
        page_size: u16,
        cursor: Option<&str>,
    ) -> Result<ListResponse<PageVersion>, ClientError> {
        let path = format!("/api/v2/pages/{id}/versions");
        self.list_page_v2(&path, page_size, cursor)
    }

    pub fn search_v2(&mut self, query: &str) -> Result<Vec<SearchResult>, ClientError> {
        let path = format!("/api/v2/search?q={}", url_encode(query));
        self.collect_all_v2(&path, 100)
    }

    pub fn search_page_v2(
        &mut self,
        query: &str,
        page_size: u16,
        cursor: Option<&str>,
    ) -> Result<ListResponse<SearchResult>, ClientError> {
        let path = format!("/api/v2/search?q={}", url_encode(query));
        self.list_page_v2(&path, page_size, cursor)
    }
}
