use super::*;

impl<A: WorkspaceApi> Client<A> {
    /// Sync the in-process CRDT document for a page.
    pub fn crdt_sync(
        &mut self,
        page_id: &str,
        state_vector: Option<&[u8]>,
    ) -> Result<(Vec<u8>, Vec<u8>), ClientError> {
        self.api
            .crdt_sync(page_id, state_vector)
            .map_err(ClientError::Realtime)
    }

    /// Apply a CRDT update through the in-process transport.
    pub fn apply_crdt_update(
        &mut self,
        page_id: &str,
        update: &[u8],
        actor: &str,
    ) -> Result<Vec<u8>, ClientError> {
        self.api
            .apply_crdt_update(page_id, update, actor)
            .map_err(ClientError::Realtime)
    }

    /// Subscribe to the in-process real-time hub, if this transport has one.
    pub fn subscribe_realtime(
        &self,
    ) -> Option<tokio::sync::broadcast::Receiver<crate::server::realtime::ServerMessage>> {
        self.api.subscribe_realtime()
    }

    /// Publish cursor/selection awareness through the in-process transport.
    pub fn publish_awareness(
        &mut self,
        awareness: &crate::server::realtime::Awareness,
    ) -> Result<(), ClientError> {
        self.api
            .publish_awareness(awareness.clone())
            .map_err(ClientError::Realtime)
    }
}
