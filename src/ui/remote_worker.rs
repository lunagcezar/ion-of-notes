//! Background REST worker for the remote desktop transport (spec-108).
//!
//! Blocking `ureq` calls run on one worker thread, so the egui thread only
//! enqueues commands and polls results. The command queue is bounded; the UI
//! coalesces search and commits before enqueueing so a fast typist cannot grow
//! an unbounded backlog.

use crate::client::Client;
use crate::collab::{MemberDraft, Share};
use crate::model::{Page, PageDraft, SearchResult, Workspace};
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
    CreatePage {
        draft: PageDraft,
        open_in_new_tab: bool,
    },
    DeletePage {
        id: String,
    },
    InviteMembers {
        drafts: Vec<MemberDraft>,
    },
    SetShare {
        share: Share,
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
    CreatePage {
        result: Result<Box<Page>, String>,
        open_in_new_tab: bool,
    },
    DeletePage {
        id: String,
        result: Result<Vec<String>, String>,
    },
    InviteMembers {
        invited: usize,
        errors: Vec<String>,
    },
    SetShare {
        share: Share,
        result: Result<Share, String>,
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
            RemoteCommand::CreatePage {
                draft,
                open_in_new_tab,
            } => RemoteResult::CreatePage {
                result: client
                    .create_page(&draft)
                    .map(Box::new)
                    .map_err(|error| error.to_string()),
                open_in_new_tab,
            },
            RemoteCommand::DeletePage { id } => {
                let result = match client.delete_page(&id) {
                    Ok(removed) => Ok(removed),
                    Err(error) => Err(error.to_string()),
                };
                RemoteResult::DeletePage { id, result }
            }
            RemoteCommand::InviteMembers { drafts } => {
                let mut invited = 0;
                let mut errors = Vec::new();
                for draft in drafts {
                    match client.invite_member(&draft) {
                        Ok(_) => invited += 1,
                        Err(error) => errors.push(error.to_string()),
                    }
                }
                RemoteResult::InviteMembers { invited, errors }
            }
            RemoteCommand::SetShare { share } => {
                let result = client.set_share(&share).map_err(|error| error.to_string());
                RemoteResult::SetShare { share, result }
            }
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
