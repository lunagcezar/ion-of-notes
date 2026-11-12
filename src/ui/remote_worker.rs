//! Background REST worker for the remote desktop transport (spec-108).
//!
//! Blocking `ureq` calls run on one worker thread, so the egui thread only
//! enqueues commands and polls results. The command queue is bounded; the UI
//! coalesces search and commits before enqueueing so a fast typist cannot grow
//! an unbounded backlog.

use crate::client::Client;
use crate::model::{Page, SearchResult, Workspace};
use crate::remote::RemoteRest;
use std::sync::mpsc::{self, Receiver, SyncSender, TrySendError};
use std::thread;

/// One blocking remote operation.
pub(super) enum RemoteCommand {
    Search {
        query: String,
    },
    SavePage {
        page: Box<Page>,
    },
    ReplaceWorkspace {
        workspace: Box<Workspace>,
    },
    CrdtSync {
        page_id: String,
    },
    CrdtUpdate {
        page_id: String,
        block_id: String,
        text: String,
        update: Vec<u8>,
    },
}

/// The result of one remote operation.
pub(super) enum RemoteResult {
    Search {
        query: String,
        result: Result<Vec<SearchResult>, String>,
    },
    SavePage {
        page_id: String,
        result: Box<Result<Page, String>>,
    },
    ReplaceWorkspace {
        result: Box<Result<Workspace, String>>,
    },
    CrdtSync {
        page_id: String,
        result: Result<(Vec<u8>, Vec<u8>), String>,
    },
    CrdtUpdate {
        page_id: String,
        block_id: String,
        text: String,
        result: Result<Vec<u8>, String>,
    },
}

/// Handle to the remote worker thread.
pub(super) struct RemoteWorker {
    commands: SyncSender<RemoteCommand>,
    results: Receiver<RemoteResult>,
}

impl RemoteWorker {
    /// Start one worker for a remote connection.
    pub(super) fn spawn(rest: RemoteRest) -> Self {
        let token = rest.token().to_string();
        let (commands, command_rx) = mpsc::sync_channel(256);
        let (result_tx, results) = mpsc::channel();
        thread::spawn(move || run(rest, token, command_rx, result_tx));
        Self { commands, results }
    }

    /// Try to enqueue a command. `false` means the bounded queue is full.
    pub(super) fn send(&self, command: RemoteCommand) -> bool {
        match self.commands.try_send(command) {
            Ok(()) => true,
            Err(TrySendError::Full(_)) => false,
            Err(TrySendError::Disconnected(_)) => false,
        }
    }

    /// Drain every finished result.
    pub(super) fn try_recv(&self) -> Option<RemoteResult> {
        self.results.try_recv().ok()
    }
}

fn run(
    rest: RemoteRest,
    token: String,
    commands: Receiver<RemoteCommand>,
    results: mpsc::Sender<RemoteResult>,
) {
    let mut client = Client::with_token(rest, token);
    while let Ok(command) = commands.recv() {
        let result = match command {
            RemoteCommand::Search { query } => RemoteResult::Search {
                result: client.search(&query).map_err(|error| error.to_string()),
                query,
            },
            RemoteCommand::SavePage { page } => {
                let page_id = page.id.clone();
                let result = client.update_page(&page).map_err(|error| error.to_string());
                RemoteResult::SavePage {
                    page_id,
                    result: Box::new(result),
                }
            }
            RemoteCommand::ReplaceWorkspace { workspace } => RemoteResult::ReplaceWorkspace {
                result: Box::new(
                    client
                        .replace_workspace(&workspace)
                        .map_err(|error| error.to_string()),
                ),
            },
            RemoteCommand::CrdtSync { page_id } => RemoteResult::CrdtSync {
                result: client
                    .crdt_sync(&page_id, None)
                    .map_err(|error| error.to_string()),
                page_id,
            },
            RemoteCommand::CrdtUpdate {
                page_id,
                block_id,
                text,
                update,
            } => RemoteResult::CrdtUpdate {
                result: client
                    .apply_crdt_update(&page_id, &update, "m-owner")
                    .map_err(|error| error.to_string()),
                page_id,
                block_id,
                text,
            },
        };
        if results.send(result).is_err() {
            break;
        }
    }
}
