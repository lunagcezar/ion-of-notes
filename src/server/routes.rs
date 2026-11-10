//! Route adapters: version dispatch and the `WorkspaceApi` implementation.

use super::realtime;
use super::{ApiRequest, ApiResponse, WorkspaceApi, WorkspaceServer};

pub(crate) mod capabilities;
mod v1;
mod v2;

impl WorkspaceApi for WorkspaceServer {
    fn handle(&mut self, request: &ApiRequest) -> ApiResponse {
        if request.path.starts_with("/api/v2") {
            self.handle_v2(request)
        } else {
            self.handle_v1(request)
        }
    }

    fn crdt_sync(
        &mut self,
        page_id: &str,
        state_vector: Option<&[u8]>,
    ) -> Result<(Vec<u8>, Vec<u8>), String> {
        WorkspaceServer::crdt_sync(self, page_id, state_vector)
    }

    fn apply_crdt_update(
        &mut self,
        page_id: &str,
        update: &[u8],
        actor: &str,
    ) -> Result<Vec<u8>, String> {
        WorkspaceServer::apply_crdt_update(self, page_id, update, actor)
    }

    fn subscribe_realtime(
        &self,
    ) -> Option<tokio::sync::broadcast::Receiver<realtime::ServerMessage>> {
        Some(self.realtime.subscribe())
    }

    fn publish_awareness(&mut self, awareness: realtime::Awareness) -> Result<(), String> {
        self.realtime.update_awareness(awareness);
        Ok(())
    }
}
