use super::*;

impl<A: RequestTransport> Client<A> {
    pub fn list_databases_v2(&mut self) -> Result<Vec<Database>, ClientError> {
        let request = self.v2_request(Method::Get, "/api/v2/databases", None)?;
        self.call_v2(request)
    }

    pub fn create_database_v2(&mut self, input: &DatabaseInput) -> Result<Database, ClientError> {
        let body = encode_body(input)?;
        let request = self.v2_request(Method::Post, "/api/v2/databases", Some(body))?;
        self.call_v2(request)
    }

    pub fn get_database_v2(&mut self, id: &str) -> Result<Database, ClientError> {
        let request = self.v2_request(Method::Get, format!("/api/v2/databases/{id}"), None)?;
        self.call_v2(request)
    }

    pub fn update_database_v2(
        &mut self,
        id: &str,
        input: &DatabaseInput,
    ) -> Result<Database, ClientError> {
        let body = encode_body(input)?;
        let request =
            self.v2_request(Method::Patch, format!("/api/v2/databases/{id}"), Some(body))?;
        self.call_v2(request)
    }

    pub fn delete_database_v2(&mut self, id: &str) -> Result<Database, ClientError> {
        let request = self.v2_request(Method::Delete, format!("/api/v2/databases/{id}"), None)?;
        self.call_v2(request)
    }

    pub fn list_data_sources_v2(
        &mut self,
        database_id: &str,
    ) -> Result<Vec<DataSource>, ClientError> {
        let request = self.v2_request(
            Method::Get,
            format!("/api/v2/databases/{database_id}/data_sources"),
            None,
        )?;
        self.call_v2(request)
    }

    pub fn create_data_source_v2(
        &mut self,
        database_id: &str,
        input: &DataSourceInput,
    ) -> Result<DataSource, ClientError> {
        let body = encode_body(input)?;
        let request = self.v2_request(
            Method::Post,
            format!("/api/v2/databases/{database_id}/data_sources"),
            Some(body),
        )?;
        self.call_v2(request)
    }

    pub fn get_data_source_v2(&mut self, id: &str) -> Result<DataSource, ClientError> {
        let request = self.v2_request(Method::Get, format!("/api/v2/data_sources/{id}"), None)?;
        self.call_v2(request)
    }

    pub fn update_data_source_v2(
        &mut self,
        id: &str,
        input: &DataSourceInput,
    ) -> Result<DataSource, ClientError> {
        let body = encode_body(input)?;
        let request = self.v2_request(
            Method::Patch,
            format!("/api/v2/data_sources/{id}"),
            Some(body),
        )?;
        self.call_v2(request)
    }

    pub fn query_data_source_page_v2(
        &mut self,
        id: &str,
        page_size: u16,
        cursor: Option<&str>,
    ) -> Result<ListResponse<Page>, ClientError> {
        let path = format!("/api/v2/data_sources/{id}/query");
        self.list_page_method_v2(Method::Post, &path, page_size, cursor)
    }

    pub fn query_data_source_v2(&mut self, id: &str) -> Result<Vec<Page>, ClientError> {
        let path = format!("/api/v2/data_sources/{id}/query");
        self.collect_all_method_v2(Method::Post, &path, 100)
    }

    pub fn page_property_items_page_v2(
        &mut self,
        page_id: &str,
        property_id: &str,
        page_size: u16,
        cursor: Option<&str>,
    ) -> Result<ListResponse<PageProperty>, ClientError> {
        let path = format!("/api/v2/pages/{page_id}/properties/{property_id}");
        self.list_page_v2(&path, page_size, cursor)
    }

    pub fn page_property_items_v2(
        &mut self,
        page_id: &str,
        property_id: &str,
    ) -> Result<Vec<PageProperty>, ClientError> {
        let path = format!("/api/v2/pages/{page_id}/properties/{property_id}");
        self.collect_all_v2(&path, 100)
    }
}
