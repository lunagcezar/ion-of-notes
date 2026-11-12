use super::*;

impl<A: RequestTransport> Client<A> {
    pub fn list_members(&mut self) -> Result<Vec<Member>, ClientError> {
        self.call(ApiRequest::new(Method::Get, "/api/members", None))
    }

    pub fn invite_member(&mut self, draft: &MemberDraft) -> Result<Member, ClientError> {
        let body = encode_body(draft)?;
        self.call(ApiRequest::new(Method::Post, "/api/members", Some(body)))
    }

    pub fn list_shares(&mut self, page_id: Option<&str>) -> Result<Vec<Share>, ClientError> {
        let path = match page_id {
            Some(id) => format!("/api/shares?page={}", url_encode(id)),
            None => "/api/shares".to_string(),
        };
        self.call(ApiRequest::new(Method::Get, path, None))
    }

    pub fn set_share(&mut self, share: &Share) -> Result<Share, ClientError> {
        let body = encode_body(share)?;
        self.call(ApiRequest::new(Method::Put, "/api/shares", Some(body)))
    }

    pub fn sync(&mut self, since: u64) -> Result<SyncBatch, ClientError> {
        self.call(ApiRequest::new(
            Method::Get,
            format!("/api/sync?since={since}"),
            None,
        ))
    }

    pub fn export_tree(&mut self) -> Result<Vec<ExportFile>, ClientError> {
        self.call(ApiRequest::new(Method::Get, "/api/export", None))
    }

    pub fn realtime_contract(&mut self) -> Result<RealtimeContract, ClientError> {
        self.call(ApiRequest::new(Method::Get, "/api/realtime", None))
    }

    // ---- v2 contract (spec-030) ------------------------------------------
    pub fn list_members_v2(&mut self) -> Result<Vec<Member>, ClientError> {
        self.collect_all_v2("/api/v2/members", 100)
    }

    pub fn list_members_page_v2(
        &mut self,
        page_size: u16,
        cursor: Option<&str>,
    ) -> Result<ListResponse<Member>, ClientError> {
        self.list_page_v2("/api/v2/members", page_size, cursor)
    }

    pub fn invite_member_v2(&mut self, draft: &MemberDraft) -> Result<Member, ClientError> {
        let body = encode_body(draft)?;
        let request = self.v2_request(Method::Post, "/api/v2/members", Some(body))?;
        self.call_v2(request)
    }

    pub fn list_shares_v2(&mut self, page_id: Option<&str>) -> Result<Vec<Share>, ClientError> {
        let path = match page_id {
            Some(id) => format!("/api/v2/shares?page={}", url_encode(id)),
            None => "/api/v2/shares".to_string(),
        };
        self.collect_all_v2(&path, 100)
    }

    pub fn list_shares_page_v2(
        &mut self,
        page_id: Option<&str>,
        page_size: u16,
        cursor: Option<&str>,
    ) -> Result<ListResponse<Share>, ClientError> {
        let path = match page_id {
            Some(id) => format!("/api/v2/shares?page={}", url_encode(id)),
            None => "/api/v2/shares".to_string(),
        };
        self.list_page_v2(&path, page_size, cursor)
    }

    pub fn set_share_v2(&mut self, share: &Share) -> Result<Share, ClientError> {
        let body = encode_body(share)?;
        let request = self.v2_request(Method::Put, "/api/v2/shares", Some(body))?;
        self.call_v2(request)
    }

    pub fn sync_v2(&mut self, since: u64) -> Result<Vec<Change>, ClientError> {
        let path = format!("/api/v2/sync?since={since}");
        self.collect_all_v2(&path, 100)
    }

    pub fn sync_page_v2(
        &mut self,
        since: u64,
        page_size: u16,
        cursor: Option<&str>,
    ) -> Result<ListResponse<Change>, ClientError> {
        let path = format!("/api/v2/sync?since={since}");
        self.list_page_v2(&path, page_size, cursor)
    }

    pub fn export_tree_v2(&mut self) -> Result<Vec<ExportFile>, ClientError> {
        let request = self.v2_request(Method::Get, "/api/v2/export", None)?;
        self.call_v2(request)
    }

    pub fn realtime_contract_v2(&mut self) -> Result<RealtimeContract, ClientError> {
        let request = self.v2_request(Method::Get, "/api/v2/realtime", None)?;
        self.call_v2(request)
    }
}
