use super::*;

impl CollabTransport for RemoteRest {
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
        None
    }

    fn publish_awareness(&mut self, _awareness: Awareness) -> Result<(), String> {
        Ok(())
    }
}

impl CollabTransport for RemoteApi {
    fn crdt_sync(
        &mut self,
        page_id: &str,
        state_vector: Option<&[u8]>,
    ) -> Result<(Vec<u8>, Vec<u8>), String> {
        self.rest.crdt_sync(page_id, state_vector)
    }

    fn apply_crdt_update(
        &mut self,
        page_id: &str,
        update: &[u8],
        actor: &str,
    ) -> Result<Vec<u8>, String> {
        self.rest.apply_crdt_update(page_id, update, actor)
    }

    fn subscribe_realtime(&self) -> Option<broadcast::Receiver<ServerMessage>> {
        self.realtime_rx
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .take()
    }

    fn publish_awareness(&mut self, awareness: Awareness) -> Result<(), String> {
        match self.ws_tx.try_send(WsCommand::Awareness(awareness)) {
            Ok(()) | Err(tokio::sync::mpsc::error::TrySendError::Full(_)) => Ok(()),
            Err(tokio::sync::mpsc::error::TrySendError::Closed(_)) => {
                Err("realtime channel closed".to_string())
            }
        }
    }
}
