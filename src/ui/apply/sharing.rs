//! Command family: sharing.

use crate::collab::{MemberDraft, Share};
use crate::i18n;

use super::super::app::NotionApp;
use super::super::commands::Command;
use super::super::invite::{is_valid_email, member_name_from_email, parse_invite_emails};
impl NotionApp {
    pub(super) fn apply_sharing(&mut self, command: Command) {
        match command {
            Command::OpenShare => {
                self.close_overlays();
                self.share_open = true;
                self.invite_error = None;
                self.invite_status = None;
                self.refresh_share();
            }
            Command::CloseShare => self.share_open = false,
            Command::SetInviteInput(input) => self.invite_input = input,
            Command::SetInviteRole(role) => self.invite_role = role,
            Command::SetInviteMessage(message) => self.invite_message = message,
            Command::SendInvites => {
                let emails: Vec<String> = parse_invite_emails(&self.invite_input)
                    .into_iter()
                    .filter(|email| is_valid_email(email))
                    .collect();
                if emails.is_empty() {
                    self.invite_error = Some(i18n::t("invalid_email"));
                    self.invite_status = None;
                } else {
                    let mut invited = 0;
                    let mut errors = Vec::new();
                    for email in emails {
                        let draft = MemberDraft {
                            name: member_name_from_email(&email),
                            email,
                            role: self.invite_role,
                        };
                        match self.client.invite_member(&draft) {
                            Ok(_) => invited += 1,
                            Err(error) => errors.push(error.to_string()),
                        }
                    }
                    if invited > 0 {
                        self.invite_input.clear();
                        self.invite_status = Some(i18n::t_count("invited_members", invited));
                    }
                    self.invite_error = if errors.is_empty() {
                        None
                    } else {
                        Some(errors.join("; "))
                    };
                    self.refresh_share();
                }
            }
            Command::SetGeneralAccess(access) => {
                let share = Share {
                    page_id: self.current_page.clone(),
                    subject: "Everyone at workspace".to_string(),
                    access,
                };
                match self.client.set_share(&share) {
                    Ok(_) => {
                        self.share_general_access = access;
                        self.invite_error = None;
                        self.invite_status = Some(i18n::t("general_updated"));
                    }
                    Err(error) => self.invite_error = Some(error.to_string()),
                }
            }
            Command::OpenInbox => {
                self.close_overlays();
                self.inbox_open = true;
            }
            Command::CloseInbox => self.inbox_open = false,
            other => debug_assert!(false, "unexpected sharing command: {other:?}"),
        }
    }
}
