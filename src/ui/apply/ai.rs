//! Command family: ai.

use crate::ollama;
use std::sync::Arc;

use super::super::app::NotionApp;
use super::super::commands::Command;
impl NotionApp {
    pub(super) fn apply_ai(&mut self, command: Command, _commands: &mut Vec<Command>) {
        match command {
            // ---- AI assistant ------------------------------------------
            Command::OpenAiChat => {
                self.close_overlays();
                self.settings_open = true;
                self.settings_page = "AI assistant".to_string();
                self.ai.answer = None;
                self.ai.error = None;
            }
            Command::SetAiQuery(query) => self.ai.query = query,
            Command::AskAi(prompt) => {
                if !self.ai.busy {
                    let config = self.ai.config.clone();
                    let slot = Arc::clone(&self.ai.slot);
                    self.ai.busy = true;
                    self.ai.started = Some(std::time::Instant::now());
                    self.ai.answer = None;
                    self.ai.error = None;
                    std::thread::spawn(move || {
                        let result = ollama::generate(&config, &prompt);
                        if let Ok(mut guard) = slot.lock() {
                            *guard = Some(result);
                        }
                    });
                }
            }
            Command::ClearAi => {
                self.ai.query.clear();
                self.ai.answer = None;
                self.ai.error = None;
                self.ai.busy = false;
                self.ai.started = None;
            }
            Command::SetAiEndpoint(endpoint) => self.ai.config.endpoint = endpoint,
            Command::SetAiModel(model) => self.ai.config.model = model,
            Command::ToggleAiCli => self.ai.config.prefer_cli = !self.ai.config.prefer_cli,
            Command::RefreshAiStatus => {
                self.ai.checking = true;
                let config = self.ai.config.clone();
                let slot = Arc::clone(&self.ai.status_slot);
                std::thread::spawn(move || {
                    let status = ollama::detect(&config);
                    if let Ok(mut guard) = slot.lock() {
                        *guard = Some(status);
                    }
                });
            }
            other => debug_assert!(false, "unexpected ai command: {other:?}"),
        }
    }
}
