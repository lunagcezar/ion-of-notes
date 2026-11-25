//! Background remote request results and dispatch (spec-108).
//!
//! The UI enqueues blocking REST/CRDT work on [`RemoteWorker`] and polls the
//! results here. Local mode keeps the synchronous path.

use crate::collab::{MemberDraft, Share};
use crate::model::{Page, PageDiff, PageDraft};
use crate::ui::remote_worker::{RemoteCommand, RemoteResult};

use super::super::app::NotionApp;
use super::super::edit_queue::CommitPlan;
use super::set_block_text;

impl NotionApp {
    /// Drain finished remote jobs and apply them on the UI thread.
    pub(crate) fn poll_remote(&mut self, now: f64) {
        let Some(worker) = &self.remote.worker else {
            return;
        };
        let mut results = Vec::new();
        while let Some(result) = worker.try_recv() {
            results.push(result);
        }
        for result in results {
            self.apply_remote_result(result, now);
        }
    }

    /// True while a commit is waiting for the worker.
    pub(crate) fn remote_commit_busy(&self) -> bool {
        self.remote.saves_in_flight > 0 || self.remote.replace_in_flight
    }

    /// Enqueue a search, keeping at most one request in flight and one queued.
    pub(crate) fn dispatch_search(&mut self, query: String) {
        if self.remote.search_in_flight.is_some() {
            self.remote.search_queued = Some(query);
            return;
        }
        let sent = {
            let Some(worker) = self.remote.worker.as_ref() else {
                return;
            };
            worker.send(RemoteCommand::Search {
                query: query.clone(),
            })
        };
        if sent {
            self.remote.search_in_flight = Some(query);
        } else {
            self.remote.search_queued = Some(query);
        }
    }

    /// Enqueue one page save.
    pub(crate) fn dispatch_save(&mut self, page: Page) -> bool {
        let sent = {
            let Some(worker) = self.remote.worker.as_ref() else {
                return false;
            };
            worker.send(RemoteCommand::SavePage {
                page: Box::new(page),
            })
        };
        if sent {
            self.remote.saves_in_flight += 1;
        }
        sent
    }

    /// Enqueue a full-workspace replace.
    pub(crate) fn dispatch_replace(&mut self, workspace: crate::model::Workspace) -> bool {
        let sent = {
            let Some(worker) = self.remote.worker.as_ref() else {
                return false;
            };
            worker.send(RemoteCommand::ReplaceWorkspace {
                workspace: Box::new(workspace),
            })
        };
        if sent {
            self.remote.replace_in_flight = true;
        }
        sent
    }

    /// Enqueue a page creation. Returns false when the worker is unavailable
    /// or its queue is full, so the caller can fall back to the sync path.
    pub(crate) fn dispatch_create_page(&mut self, draft: PageDraft, open_in_new_tab: bool) -> bool {
        let Some(worker) = self.remote.worker.as_ref() else {
            return false;
        };
        worker.send(RemoteCommand::CreatePage {
            draft,
            open_in_new_tab,
        })
    }

    /// Enqueue a page deletion.
    pub(crate) fn dispatch_delete_page(&mut self, id: String) -> bool {
        let Some(worker) = self.remote.worker.as_ref() else {
            return false;
        };
        worker.send(RemoteCommand::DeletePage { id })
    }

    /// Enqueue one invite batch.
    pub(crate) fn dispatch_invite_members(&mut self, drafts: Vec<MemberDraft>) -> bool {
        let Some(worker) = self.remote.worker.as_ref() else {
            return false;
        };
        worker.send(RemoteCommand::InviteMembers { drafts })
    }

    /// Enqueue one permission-grant update.
    pub(crate) fn dispatch_set_share(&mut self, share: Share) -> bool {
        let Some(worker) = self.remote.worker.as_ref() else {
            return false;
        };
        worker.send(RemoteCommand::SetShare { share })
    }

    /// Try to send one page's structure/property/row diff through CRDT.
    ///
    /// Returns false when the diff includes page metadata or data-source
    /// schema, or when the worker queue is full, so the caller uses REST.
    pub(crate) fn try_dispatch_crdt_commit(&mut self, diff: &PageDiff) -> bool {
        if !self.remote.is_remote()
            || !diff.added.is_empty()
            || !diff.removed.is_empty()
            || diff.reordered
            || diff.settings_changed
            || diff.changed.len() != 1
        {
            return false;
        }
        let page_id = diff.changed[0].clone();
        if page_id != self.collaboration.page {
            return false;
        }
        let Some(old) = self.last_committed.page(&page_id).cloned() else {
            return false;
        };
        let Some(new) = self.workspace.page(&page_id).cloned() else {
            return false;
        };
        let update = match self.collaboration.crdt.plan_page_changes(
            &old,
            &new,
            &self.last_committed,
            &self.workspace,
        ) {
            Ok(Some(update)) => update,
            _ => return false,
        };
        if self.dispatch_crdt_update(page_id, String::new(), String::new(), update) {
            self.last_committed = self.workspace.clone();
            self.pending_edits.flushed();
            true
        } else {
            false
        }
    }

    /// Enqueue a commit plan; the pending edit is retried if the queue is full.
    pub(crate) fn dispatch_plan(&mut self, plan: &CommitPlan) {
        match plan {
            CommitPlan::None => {}
            CommitPlan::Update(ids) => {
                let pages: Vec<Page> = ids
                    .iter()
                    .filter_map(|id| self.workspace.page(id).cloned())
                    .collect();
                for page in pages {
                    self.dispatch_save(page);
                }
            }
            CommitPlan::Replace => {
                self.dispatch_replace(self.workspace.clone());
            }
        }
    }

    /// Enqueue a CRDT page sync. Returns false when the worker is unavailable.
    pub(crate) fn dispatch_crdt_sync(&mut self, page_id: String) -> bool {
        let sent = {
            let Some(worker) = self.remote.worker.as_ref() else {
                return false;
            };
            worker.send(RemoteCommand::CrdtSync {
                page_id: page_id.clone(),
            })
        };
        if sent {
            self.remote.crdt_sync_in_flight = Some(page_id);
        }
        sent
    }

    /// Enqueue a CRDT text update. Returns false when it could not be queued.
    pub(crate) fn dispatch_crdt_update(
        &mut self,
        page_id: String,
        block_id: String,
        text: String,
        update: Vec<u8>,
    ) -> bool {
        let sent = {
            let Some(worker) = self.remote.worker.as_ref() else {
                return false;
            };
            worker.send(RemoteCommand::CrdtUpdate {
                page_id,
                block_id,
                text,
                update,
            })
        };
        if sent {
            self.remote.crdt_updates_in_flight += 1;
        }
        sent
    }

    fn apply_remote_result(&mut self, result: RemoteResult, now: f64) {
        match result {
            RemoteResult::Search { query, result } => {
                if self.remote.search_in_flight.as_deref() == Some(query.as_str()) {
                    self.remote.search_in_flight = None;
                }
                if self.search.query == query
                    && let Ok(results) = result
                {
                    self.search.results = std::sync::Arc::new(results);
                }
                if let Some(queued) = self.remote.search_queued.take()
                    && queued != query
                {
                    self.dispatch_search(queued);
                }
            }
            RemoteResult::SavePage { page_id, result } => {
                self.remote.saves_in_flight = self.remote.saves_in_flight.saturating_sub(1);
                if let Ok(saved) = *result
                    && let Some(committed) = self.last_committed.page_mut(&page_id)
                {
                    *committed = saved;
                }
                if !self.remote_commit_busy() {
                    self.finish_remote_commit(now);
                }
            }
            RemoteResult::ReplaceWorkspace { result } => {
                self.remote.replace_in_flight = false;
                if let Ok(saved) = *result {
                    self.last_committed = saved;
                }
                self.finish_remote_commit(now);
            }
            RemoteResult::CreatePage {
                result,
                open_in_new_tab,
            } => match result {
                Ok(page) => {
                    let page = *page;
                    if open_in_new_tab {
                        self.open_in_tab(page.id.clone());
                    } else {
                        self.current_page = page.id.clone();
                        if let Some(tab) = self.tabs.get_mut(self.active_tab) {
                            *tab = page.id.clone();
                        }
                        self.library_open = false;
                        self.close_overlays();
                        self.push_history(page.id.clone());
                    }
                    self.pending_reload = true;
                }
                Err(error) => self.page_status = Some(error),
            },
            RemoteResult::DeletePage { id, result } => match result {
                Ok(_) => self.pending_reload = true,
                Err(error) => self.page_status = Some(format!("Could not delete {id}: {error}")),
            },
            RemoteResult::InviteMembers { invited, errors } => {
                self.finish_invites(invited, errors);
            }
            RemoteResult::SetShare { share, result } => {
                self.finish_share(&share, result);
            }
            RemoteResult::CrdtSync { page_id, result } => {
                if self.remote.crdt_sync_in_flight.as_deref() == Some(page_id.as_str()) {
                    self.remote.crdt_sync_in_flight = None;
                }
                if page_id != self.collaboration.page {
                    return;
                }
                match result {
                    Ok((update, _)) => {
                        if self.collaboration.crdt.apply_update(&update).is_ok() {
                            if let Some(page) = self.workspace.page(&page_id).cloned() {
                                self.collaboration.crdt.baseline_page(&page);
                            }
                            self.project_crdt_page();
                        }
                    }
                    Err(_) => {
                        if let Some(page) = self.workspace.page(&page_id).cloned() {
                            self.collaboration
                                .crdt
                                .open_page_with_workspace(&page, &self.workspace);
                        }
                    }
                }
            }
            RemoteResult::CrdtUpdate {
                page_id,
                block_id,
                text,
                result,
            } => {
                self.remote.crdt_updates_in_flight =
                    self.remote.crdt_updates_in_flight.saturating_sub(1);
                if result.is_ok()
                    && let Some(page) = self.last_committed.page_mut(&page_id)
                {
                    set_block_text(&mut page.blocks, &block_id, &text);
                }
            }
        }
    }

    fn finish_remote_commit(&mut self, now: f64) {
        if !self.pending_edits.is_pending() {
            return;
        }
        if self.workspace == self.last_committed {
            self.pending_edits.flushed();
        } else {
            self.pending_edits.mark(now);
        }
    }
}
