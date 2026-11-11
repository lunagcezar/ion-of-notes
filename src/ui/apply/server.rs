//! Command family: server.

use super::super::app::NotionApp;
use super::super::commands::Command;
impl NotionApp {
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
                let url = self.server_url.trim().trim_end_matches('/').to_string();
                let email = self.server_email.trim().to_string();
                let password = self.server_password.clone();
                if url.is_empty() || email.is_empty() || password.is_empty() {
                    self.server_status =
                        Some("Server, email and password are required.".to_string());
                } else {
                    match crate::remote::login(&url, &email, &password) {
                        Ok(session) => {
                            match crate::app::build_remote_desktop(&url, &session.token) {
                                Ok((client, workspace)) => {
                                    self.replace_client(client, workspace);
                                    self.server_connected = true;
                                    self.server_password.clear();
                                    self.server_status = Some(format!("Connected to {url}"));
                                }
                                Err(error) => {
                                    self.server_status = Some(error);
                                }
                            }
                        }
                        Err(error) => {
                            self.server_status = Some(error);
                        }
                    }
                }
            }
            Command::DisconnectServer => {
                let (client, workspace) = crate::app::build_desktop();
                self.replace_client(client, workspace);
                self.server_connected = false;
                self.server_password.clear();
                self.server_status = Some("Using the local workspace.".to_string());
            }
            other => debug_assert!(false, "unexpected server command: {other:?}"),
        }
    }
}
