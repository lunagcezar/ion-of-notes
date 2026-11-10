//! Legacy v1 route dispatch. Every route keeps the v1 envelope and status.

use super::super::response::{bad, json, not_found, parse_body, query_value, server_error};
use super::super::{ApiRequest, ApiResponse, Envelope, Method, WorkspaceServer};
use crate::collab::{ChangeOp, MemberDraft, Share, export_tree, realtime_contract};
use crate::model::{Page, PageDraft, PageSummary, SearchResult, Workspace};
use crate::webhooks::WebhookEventType;

impl WorkspaceServer {
    pub(super) fn handle_v1(&mut self, request: &ApiRequest) -> ApiResponse {
        let (path, query) = match request.path.split_once('?') {
            Some((p, q)) => (p, q),
            None => (request.path.as_str(), ""),
        };

        match (request.method, path) {
            (Method::Get, "/health") => json(200, Envelope::ok("ok".to_string())),

            (Method::Get, "/api/workspace") => json(200, Envelope::ok(self.workspace.clone())),
            (Method::Put, "/api/workspace") => match parse_body::<Workspace>(request) {
                Ok(workspace) => {
                    let actor = self.collab.owner_id();
                    for page in &workspace.pages {
                        let changed = self
                            .workspace
                            .page(&page.id)
                            .map(|old| old != page)
                            .unwrap_or(true);
                        if changed {
                            self.collab.record_version(page, &actor);
                        }
                    }
                    self.workspace = workspace;
                    if let Err(error) = self.write(ChangeOp::Replace, "__workspace__") {
                        return server_error(&error.to_string());
                    }
                    json(200, Envelope::ok(self.workspace.clone()))
                }
                Err(message) => bad(&message),
            },

            (Method::Get, "/api/pages") => {
                let summaries: Vec<PageSummary> =
                    self.workspace.pages.iter().map(|p| p.summary()).collect();
                json(200, Envelope::ok(summaries))
            }
            (Method::Post, "/api/pages") => match parse_body::<PageDraft>(request) {
                Ok(draft) => {
                    let title = draft.title.trim().to_string();
                    if title.is_empty() {
                        return bad("page title must not be empty");
                    }
                    if let Some(parent) = draft.parent.as_deref()
                        && self.workspace.page(parent).is_none()
                    {
                        return bad("unknown parent page");
                    }
                    let icon = if draft.icon.trim().is_empty() {
                        "P".to_string()
                    } else {
                        draft.icon.clone()
                    };
                    let mut page = Page::new(uuid::Uuid::new_v4().to_string(), title, icon);
                    page.parent = draft.parent;
                    if let Err(e) = self.workspace.add_page(page.clone()) {
                        return bad(&e.to_string());
                    }
                    let page_id = page.id.clone();
                    let actor = self.collab.owner_id();
                    self.collab.record_version(&page, &actor);
                    if let Err(error) = self.write(ChangeOp::Create, &page_id) {
                        return server_error(&error.to_string());
                    }
                    self.emit_webhook(
                        WebhookEventType::PageCreated,
                        "page",
                        &page_id,
                        serde_json::json!({ "parent": page.parent }).to_string(),
                    );
                    json(201, Envelope::ok(page))
                }
                Err(message) => bad(&message),
            },

            (Method::Get, "/api/search") => {
                let q = query_value(query, "q");
                let results: Vec<SearchResult> = self.workspace.search(&q);
                json(200, Envelope::ok(results))
            }

            // ---- decided collaborative API (see docs/adr-003-*) ----------
            (Method::Get, "/api/members") => json(200, Envelope::ok(self.collab.members.clone())),
            (Method::Post, "/api/members") => match parse_body::<MemberDraft>(request) {
                Ok(draft) => {
                    if draft.name.trim().is_empty() || draft.email.trim().is_empty() {
                        return bad("member name and email must not be empty");
                    }
                    let member = self.collab.invite(draft);
                    if let Err(error) = self.persist_collab() {
                        return server_error(&error.to_string());
                    }
                    json(201, Envelope::ok(member))
                }
                Err(message) => bad(&message),
            },
            (Method::Get, "/api/shares") => {
                let page = query_value(query, "page");
                let shares = if page.is_empty() {
                    self.collab.shares.clone()
                } else {
                    self.collab.shares_for(&page)
                };
                json(200, Envelope::ok(shares))
            }
            (Method::Put, "/api/shares") => match parse_body::<Share>(request) {
                Ok(share) => {
                    self.collab.set_access(share.clone());
                    if let Err(error) = self.persist_collab() {
                        return server_error(&error.to_string());
                    }
                    json(200, Envelope::ok(share))
                }
                Err(message) => bad(&message),
            },
            (Method::Get, "/api/sync") => {
                let since = query_value(query, "since").parse::<u64>().unwrap_or(0);
                json(200, Envelope::ok(self.collab.changes_since(since)))
            }
            (Method::Get, "/api/export") => json(
                200,
                Envelope::ok(export_tree(&self.workspace, &self.collab.members)),
            ),
            (Method::Get, "/api/realtime") => json(200, Envelope::ok(realtime_contract())),
            // ---------------------------------------------------------------
            (method, page_path) if page_path.starts_with("/api/pages/") => {
                let rest = &page_path["/api/pages/".len()..];
                if rest.is_empty() {
                    return not_found("unknown route");
                }
                if let Some(id) = rest.strip_suffix("/versions") {
                    if id.is_empty() {
                        return not_found("unknown route");
                    }
                    return match method {
                        Method::Get => json(200, Envelope::ok(self.collab.versions_for(id))),
                        _ => not_found("unknown route"),
                    };
                }
                let id = rest;
                match method {
                    Method::Get => match self.workspace.page(id) {
                        Some(page) => json(200, Envelope::ok(page.clone())),
                        None => not_found("page not found"),
                    },
                    Method::Patch => match parse_body::<Page>(request) {
                        Ok(mut page) => {
                            page.id = id.to_string();
                            let was_locked = self
                                .workspace
                                .page(id)
                                .map(|existing| existing.locked)
                                .unwrap_or(false);
                            match self.workspace.page_mut(id) {
                                Some(slot) => {
                                    *slot = page.clone();
                                    let actor = self.collab.owner_id();
                                    self.collab.record_version(&page, &actor);
                                    if let Err(error) = self.write(ChangeOp::Update, id) {
                                        return server_error(&error.to_string());
                                    }
                                    if was_locked != page.locked {
                                        self.emit_webhook(
                                            if page.locked {
                                                WebhookEventType::PageLocked
                                            } else {
                                                WebhookEventType::PageUnlocked
                                            },
                                            "page",
                                            id,
                                            serde_json::json!({}).to_string(),
                                        );
                                    } else {
                                        self.emit_webhook(
                                            WebhookEventType::PageContentUpdated,
                                            "page",
                                            id,
                                            serde_json::json!({}).to_string(),
                                        );
                                    }
                                    json(200, Envelope::ok(page))
                                }
                                None => not_found("page not found"),
                            }
                        }
                        Err(message) => bad(&message),
                    },
                    Method::Delete => match self.workspace.remove_page(id) {
                        Ok(removed) => {
                            if let Err(error) = self.write(ChangeOp::Delete, id) {
                                return server_error(&error.to_string());
                            }
                            self.emit_webhook(
                                WebhookEventType::PageDeleted,
                                "page",
                                id,
                                serde_json::json!({}).to_string(),
                            );
                            json(200, Envelope::ok(removed))
                        }
                        Err(e) => not_found(&e.to_string()),
                    },
                    _ => not_found("unknown route"),
                }
            }

            _ => not_found("unknown route"),
        }
    }
}
