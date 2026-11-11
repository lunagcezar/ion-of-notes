//! Command family: server connection and login.

use std::sync::Arc;

use super::super::app::NotionApp;
use super::super::commands::Command;

impl NotionApp {
    /// Start a background login/connect. The UI keeps rendering while the
    /// network call runs; `poll_server` applies the result on a later frame.
    fn start_server_connect(
        &mut self,
        url: String,
        email: String,
        password: Option<String>,
        token: Option<String>,
    ) {
        if self.server_busy {
            return;
        }
        self.server_busy = true;
        self.server_status = Some("Connecting...".to_string());
        let slot = Arc::clone(&self.server_slot);
        std::thread::spawn(move || {
            let result = if let Some(token) = token {
                crate::app::build_remote_desktop(&url, &token)
                    .map(|(client, workspace)| (client, workspace, url.clone()))
            } else {
                let password = password.unwrap_or_default();
                crate::remote::login(&url, &email, &password).and_then(|session| {
                    let (client, workspace) =
                        crate::app::build_remote_desktop(&url, &session.token)?;
                    let _ = crate::remote::save_session(&url, &email, &session.token);
                    Ok((client, workspace, url.clone()))
                })
            };
            if let Ok(mut guard) = slot.lock() {
                *guard = Some(result);
            }
        });
    }

    /// Connect with an explicit bearer token without blocking startup.
    pub fn connect_server_with_token(&mut self, server: String, token: String) {
        self.server_url = server.clone();
        self.start_server_connect(server, self.server_email.clone(), None, Some(token));
    }

    /// Restore a remembered session for `server`, or open the login page when
    /// there is none. The actual connection stays off the UI thread.
    pub fn restore_server_session(&mut self, server: String) {
        self.server_url = server.clone();
        if let Some(session) = crate::remote::load_session(&server) {
            self.server_email = session.email;
            self.start_server_connect(server, self.server_email.clone(), None, Some(session.token));
        } else {
            self.open_server_login(server);
        }
    }

    /// Apply a finished background connection.
    pub(crate) fn poll_server(&mut self) {
        if !self.server_busy {
            return;
        }
        let done = self
            .server_slot
            .lock()
            .ok()
            .and_then(|mut guard| guard.take());
        let Some(result) = done else {
            return;
        };
        self.server_busy = false;
        match result {
            Ok((client, workspace, url)) => {
                self.replace_client(client, workspace);
                self.server_connected = true;
                self.server_password.clear();
                self.server_status = Some(format!("Connected to {url}"));
            }
            Err(error) => {
                let _ = crate::remote::clear_session();
                self.server_status = Some(error);
                self.settings_open = true;
                self.settings_page = "Server".to_string();
            }
        }
    }

    pub(super) fn apply_server(&mut self, command: Command) {
        match command {
            Command::SetServerUrl(value) => {
                self.server_url = value;
                self.server_status = None;
            }
            Command::SetServerEmail(value) => {
                self.server_email = value;
                self.server_status = None;
            }
            Command::SetServerPassword(value) => {
                self.server_password = value;
                self.server_status = None;
            }
            Command::ConnectServer => {
                if self.server_busy {
                    return;
                }
                let url = self.server_url.trim().trim_end_matches('/').to_string();
                let email = self.server_email.trim().to_string();
                let password = self.server_password.clone();
                if url.is_empty() || email.is_empty() || password.is_empty() {
                    self.server_status =
                        Some("Server, email and password are required.".to_string());
                } else {
                    self.start_server_connect(url, email, Some(password), None);
                }
            }
            Command::DisconnectServer => {
                let _ = crate::remote::clear_session();
                let (client, workspace) = crate::app::build_desktop();
                self.replace_client(client, workspace);
                self.server_busy = false;
                self.server_connected = false;
                self.server_password.clear();
                self.server_status = Some("Using the local workspace.".to_string());
            }
            other => debug_assert!(false, "unexpected server command: {other:?}"),
        }
    }
}
