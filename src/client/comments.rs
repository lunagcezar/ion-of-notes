use super::*;

impl<A: RequestTransport> Client<A> {
    fn comments_path(page: Option<&str>, block: Option<&str>) -> String {
        match (page, block) {
            (Some(page), _) => format!("/api/v2/comments?page={}", url_encode(page)),
            (None, Some(block)) => format!("/api/v2/comments?block={}", url_encode(block)),
            (None, None) => "/api/v2/comments".to_string(),
        }
    }

    pub fn list_comments_page_v2(
        &mut self,
        page: Option<&str>,
        block: Option<&str>,
        page_size: u16,
        cursor: Option<&str>,
    ) -> Result<ListResponse<CommentDto>, ClientError> {
        let path = Self::comments_path(page, block);
        self.list_page_v2(&path, page_size, cursor)
    }

    pub fn list_comments_v2(
        &mut self,
        page: Option<&str>,
        block: Option<&str>,
    ) -> Result<Vec<CommentDto>, ClientError> {
        let path = Self::comments_path(page, block);
        self.collect_all_v2(&path, 100)
    }

    pub fn create_comment_v2(&mut self, input: &CommentInput) -> Result<CommentDto, ClientError> {
        let body = encode_body(input)?;
        let request = self.v2_request(Method::Post, "/api/v2/comments", Some(body))?;
        self.call_v2(request)
    }

    pub fn get_comment_v2(&mut self, id: &str) -> Result<CommentDto, ClientError> {
        let request = self.v2_request(Method::Get, format!("/api/v2/comments/{id}"), None)?;
        self.call_v2(request)
    }

    pub fn update_comment_v2(
        &mut self,
        id: &str,
        input: &CommentInput,
    ) -> Result<CommentDto, ClientError> {
        let body = encode_body(input)?;
        let request =
            self.v2_request(Method::Patch, format!("/api/v2/comments/{id}"), Some(body))?;
        self.call_v2(request)
    }

    pub fn delete_comment_v2(&mut self, id: &str) -> Result<CommentDto, ClientError> {
        let request = self.v2_request(Method::Delete, format!("/api/v2/comments/{id}"), None)?;
        self.call_v2(request)
    }

    pub fn list_users_v2(&mut self) -> Result<Vec<UserDto>, ClientError> {
        let request = self.v2_request(Method::Get, "/api/v2/users", None)?;
        self.call_v2(request)
    }

    pub fn get_user_v2(&mut self, id: &str) -> Result<UserDto, ClientError> {
        let request = self.v2_request(Method::Get, format!("/api/v2/users/{id}"), None)?;
        self.call_v2(request)
    }

    pub fn create_file_upload_v2(
        &mut self,
        input: &FileUploadInput,
    ) -> Result<FileUpload, ClientError> {
        let body = encode_body(input)?;
        let request = self.v2_request(Method::Post, "/api/v2/file_uploads", Some(body))?;
        self.call_v2(request)
    }

    pub fn send_file_upload_v2(
        &mut self,
        id: &str,
        content: &str,
    ) -> Result<FileUpload, ClientError> {
        let request = self.v2_request(
            Method::Post,
            format!("/api/v2/file_uploads/{id}/send"),
            Some(content.to_string()),
        )?;
        self.call_v2(request)
    }

    pub fn complete_file_upload_v2(&mut self, id: &str) -> Result<FileUpload, ClientError> {
        let request = self.v2_request(
            Method::Post,
            format!("/api/v2/file_uploads/{id}/complete"),
            None,
        )?;
        self.call_v2(request)
    }

    pub fn get_file_upload_v2(&mut self, id: &str) -> Result<FileUpload, ClientError> {
        let request = self.v2_request(Method::Get, format!("/api/v2/file_uploads/{id}"), None)?;
        self.call_v2(request)
    }

    pub fn list_file_uploads_page_v2(
        &mut self,
        page_size: u16,
        cursor: Option<&str>,
    ) -> Result<ListResponse<FileUpload>, ClientError> {
        self.list_page_v2("/api/v2/file_uploads", page_size, cursor)
    }

    pub fn list_file_uploads_v2(&mut self) -> Result<Vec<FileUpload>, ClientError> {
        self.collect_all_v2("/api/v2/file_uploads", 100)
    }
}
