//! Command family: ai.

use crate::ollama;
use std::sync::Arc;

use super::super::app::NotionApp;
use super::super::commands::Command;
impl NotionApp {
    pub(super) fn apply_ai(&mut self, command: Command, commands: &mut Vec<Command>) {
        match command {
            // ---- AI assistant ------------------------------------------
            Command::OpenAiChat => {
                self.close_overlays();
                self.search_open = true;
                self.ai_answer = None;
                self.ai_error = None;
                if !self.ai_query.trim().is_empty() {
                    commands.push(Command::AskAi(self.ai_query.clone()));
                }
            }
            Command::SetAiQuery(query) => self.ai_query = query,
            Command::AskAi(prompt) => {
                if !self.ai_busy {
                    let config = self.ai_config.clone();
                    let slot = Arc::clone(&self.ai_slot);
                    self.ai_busy = true;
                    self.ai_started = Some(std::time::Instant::now());
                    self.ai_answer = None;
                    self.ai_error = None;
                    std::thread::spawn(move || {
                        let result = ollama::generate(&config, &prompt);
                        if let Ok(mut guard) = slot.lock() {
                            *guard = Some(result);
                        }
                    });
                }
            }
            Command::ClearAi => {
                self.ai_query.clear();
                self.ai_answer = None;
                self.ai_error = None;
                self.ai_busy = false;
                self.ai_started = None;
            }
            Command::SetAiEndpoint(endpoint) => self.ai_config.endpoint = endpoint,
            Command::SetAiModel(model) => self.ai_config.model = model,
            Command::ToggleAiCli => self.ai_config.prefer_cli = !self.ai_config.prefer_cli,
            Command::RefreshAiStatus => {
                self.ai_status = ollama::detect(&self.ai_config);
            }
            other => debug_assert!(false, "unexpected ai command: {other:?}"),
        }
    }
}
