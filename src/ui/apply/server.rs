//! Command family: server connection and login.

use std::sync::Arc;

use super::super::app::NotionApp;
use super::super::commands::Command;

impl NotionApp {
    fn remote_rest(&self) -> Option<crate::remote::RemoteRest> {
        match self.client.api() {
            crate::app::DesktopApi::Remote(remote) => Some(remote.rest()),
            crate::app::DesktopApi::Local(_) => None,
        }
    }

    fn remote_server_url(&self) -> Option<String> {
        self.remote_rest()
            .map(|_| self.server.url.trim_end_matches('/').to_string())
    }
    /// Start a background login/connect. The UI keeps rendering while the
    /// network call runs; `poll_server` applies the result on a later frame.
    fn start_server_connect(
        &mut self,
        url: String,
        email: String,
        password: Option<String>,
        token: Option<String>,
    ) {
        if self.server.busy {
            return;
        }
        self.server.busy = true;
        self.server.status = Some("Connecting...".to_string());
        let slot = Arc::clone(&self.server.slot);
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
        self.server.url = server.clone();
        self.start_server_connect(server, self.server.email.clone(), None, Some(token));
    }

    /// Restore a remembered session for `server`, or open the login page when
    /// there is none. The actual connection stays off the UI thread.
    pub fn restore_server_session(&mut self, server: String) {
        self.server.url = server.clone();
        if let Some(session) = crate::remote::load_session(&server) {
            self.server.email = session.email;
            self.start_server_connect(server, self.server.email.clone(), None, Some(session.token));
        } else {
            self.open_server_login(server);
        }
    }

    /// Apply a finished background connection.
    pub(crate) fn poll_server(&mut self) {
        if !self.server.busy {
            return;
        }
        let done = self
            .server
            .slot
            .lock()
            .ok()
            .and_then(|mut guard| guard.take());
        let Some(result) = done else {
            return;
        };
        self.server.busy = false;
        match result {
            Ok((client, workspace, url)) => {
                self.replace_client(client, workspace);
                self.server.connected = true;
                self.server.password.clear();
                self.server.status = Some(format!("Connected to {url}"));
            }
            Err(error) => {
                let _ = crate::remote::clear_session();
                self.server.status = Some(error);
                self.settings.open = true;
                self.settings.page = "Server".to_string();
            }
        }
    }

    pub(super) fn apply_server(&mut self, command: Command) {
        match command {
            Command::SetServerUrl(value) => {
                self.server.url = value;
                self.server.status = None;
            }
            Command::SetServerEmail(value) => {
                self.server.email = value;
                self.server.status = None;
            }
            Command::SetServerPassword(value) => {
                self.server.password = value;
                self.server.status = None;
            }
            Command::SetServerRegister(value) => {
                self.server.register = value;
                self.server.status = None;
            }
            Command::SetServerCurrentPassword(value) => {
                self.server.current_password = value;
                self.server.status = None;
            }
            Command::SetServerNewPassword(value) => {
                self.server.new_password = value;
                self.server.status = None;
            }
            Command::SetServerConfirmPassword(value) => {
                self.server.confirm_password = value;
                self.server.status = None;
            }
            Command::SetServerResetEmail(value) => {
                self.server.reset_email = value;
                self.server.status = None;
            }
            Command::SetServerResetPassword(value) => {
                self.server.reset_password = value;
                self.server.status = None;
            }
            Command::ChangeServerPassword => {
                let Some(url) = self.remote_server_url() else {
                    self.server.status = Some("Connect to a server first.".to_string());
                    return;
                };
                if self.server.new_password != self.server.confirm_password {
                    self.server.status = Some("The new passwords do not match.".to_string());
                    return;
                }
                let Some(rest) = self.remote_rest() else {
                    self.server.status = Some("Not connected to a server.".to_string());
                    return;
                };
                match rest.change_password(&self.server.current_password, &self.server.new_password)
                {
                    Ok(_) => {
                        let password = self.server.new_password.clone();
                        self.server.current_password.clear();
                        self.server.new_password.clear();
                        self.server.confirm_password.clear();
                        self.server.status = Some("Password changed. Reconnecting...".to_string());
                        self.start_server_connect(
                            url,
                            self.server.email.clone(),
                            Some(password),
                            None,
                        );
                    }
                    Err(error) => self.server.status = Some(error),
                }
            }
            Command::ResetServerPassword => {
                let Some(rest) = self.remote_rest() else {
                    self.server.status = Some("Not connected to a server.".to_string());
                    return;
                };
                match rest.reset_password(&self.server.reset_email, &self.server.reset_password) {
                    Ok(()) => {
                        self.server.reset_password.clear();
                        self.server.status = Some("Member password reset.".to_string());
                    }
                    Err(error) => self.server.status = Some(error),
                }
            }
            Command::ConnectServer => {
                if self.server.busy {
                    return;
                }
                let url = self.server.url.trim().trim_end_matches('/').to_string();
                let email = self.server.email.trim().to_string();
                let password = self.server.password.clone();
                if url.is_empty() || email.is_empty() || password.is_empty() {
                    self.server.status =
                        Some("Server, email and password are required.".to_string());
                } else if self.server.register {
                    match crate::remote::register_account(&url, &email, &password) {
                        Ok(_) => {
                            self.start_server_connect(url, email, Some(password), None);
                        }
                        Err(error) => self.server.status = Some(error),
                    }
                } else {
                    self.start_server_connect(url, email, Some(password), None);
                }
            }
            Command::DisconnectServer => {
                let _ = crate::remote::clear_session();
                let (client, workspace, catalog) = crate::app::build_desktop_catalog()
                    .unwrap_or_else(|error| {
                        eprintln!("could not reload the workspace ({error}); staying in memory");
                        crate::app::fallback_desktop()
                    });
                self.catalog = catalog;
                self.catalog_path = Some(crate::store::catalog_path());
                self.replace_client(client, workspace);
                self.server.busy = false;
                self.server.connected = false;
                self.server.password.clear();
                self.server.status = Some("Using the local workspace.".to_string());
            }
            other => debug_assert!(false, "unexpected server command: {other:?}"),
        }
    }
}
