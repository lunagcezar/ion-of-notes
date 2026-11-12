use super::*;

impl CollabTransport for RemoteApi {
    fn crdt_sync(
        &mut self,
        page_id: &str,
        state_vector: Option<&[u8]>,
    ) -> Result<(Vec<u8>, Vec<u8>), String> {
        let output: CrdtSyncOutput = self.post_json(
            "/api/v2/crdt/sync",
            &CrdtSyncInput {
                page_id: page_id.to_string(),
                state_vector: state_vector.map(|bytes| BASE64.encode(bytes)),
            },
        )?;
        let update = BASE64
            .decode(output.update)
            .map_err(|error| error.to_string())?;
        let vector = BASE64
            .decode(output.state_vector)
            .map_err(|error| error.to_string())?;
        Ok((update, vector))
    }

    fn apply_crdt_update(
        &mut self,
        page_id: &str,
        update: &[u8],
        actor: &str,
    ) -> Result<Vec<u8>, String> {
        let output: CrdtUpdateOutput = self.post_json(
            "/api/v2/crdt/update",
            &CrdtUpdateInput {
                page_id: page_id.to_string(),
                update: BASE64.encode(update),
                actor: Some(actor.to_string()),
            },
        )?;
        BASE64
            .decode(output.update)
            .map_err(|error| error.to_string())
    }

    fn subscribe_realtime(&self) -> Option<broadcast::Receiver<ServerMessage>> {
        self.realtime_rx
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .take()
    }

    fn publish_awareness(&mut self, awareness: Awareness) -> Result<(), String> {
        self.ws_tx
            .send(WsCommand::Awareness(awareness))
            .map_err(|error| error.to_string())
    }
}
