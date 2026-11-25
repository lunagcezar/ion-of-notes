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
                self.sharing.open = true;
                self.sharing.invite_error = None;
                self.sharing.invite_status = None;
                self.refresh_share();
            }
            Command::CloseShare => self.sharing.open = false,
            Command::SetInviteInput(input) => self.sharing.invite_input = input,
            Command::SetInviteRole(role) => self.sharing.invite_role = role,
            Command::SetInviteMessage(message) => self.sharing.invite_message = message,
            Command::SendInvites => {
                let emails: Vec<String> = parse_invite_emails(&self.sharing.invite_input)
                    .into_iter()
                    .filter(|email| is_valid_email(email))
                    .collect();
                if emails.is_empty() {
                    self.sharing.invite_error = Some(i18n::t("invalid_email"));
                    self.sharing.invite_status = None;
                } else {
                    let drafts: Vec<MemberDraft> = emails
                        .into_iter()
                        .map(|email| MemberDraft {
                            name: member_name_from_email(&email),
                            email,
                            role: self.sharing.invite_role,
                        })
                        .collect();
                    if self.remote.is_remote() && self.dispatch_invite_members(drafts.clone()) {
                        return;
                    }
                    let mut invited = 0;
                    let mut errors = Vec::new();
                    for draft in drafts {
                        match self.client.invite_member(&draft) {
                            Ok(_) => invited += 1,
                            Err(error) => errors.push(error.to_string()),
                        }
                    }
                    self.finish_invites(invited, errors);
                }
            }
            Command::SetGeneralAccess(access) => {
                let share = Share {
                    page_id: self.current_page.clone(),
                    subject: "Everyone at workspace".to_string(),
                    access,
                };
                if self.remote.is_remote() && self.dispatch_set_share(share.clone()) {
                    return;
                }
                let result = self
                    .client
                    .set_share(&share)
                    .map_err(|error| error.to_string());
                self.finish_share(&share, result);
            }
            Command::OpenInbox => {
                self.close_overlays();
                self.inbox_open = true;
            }
            Command::CloseInbox => self.inbox_open = false,
            other => debug_assert!(false, "unexpected sharing command: {other:?}"),
        }
    }

    /// Apply an invite batch result from the sync or async path.
    pub(super) fn finish_invites(&mut self, invited: usize, errors: Vec<String>) {
        if invited > 0 {
            self.sharing.invite_input.clear();
            self.sharing.invite_status = Some(i18n::t_count("invited_members", invited));
        }
        self.sharing.invite_error = if errors.is_empty() {
            None
        } else {
            Some(errors.join("; "))
        };
        self.refresh_share();
    }

    /// Apply one permission-grant result from the sync or async path.
    pub(super) fn finish_share(&mut self, share: &Share, result: Result<Share, String>) {
        match result {
            Ok(_) => {
                self.sharing.general_access = share.access;
                self.sharing.invite_error = None;
                self.sharing.invite_status = Some(i18n::t("general_updated"));
            }
            Err(error) => self.sharing.invite_error = Some(error),
        }
    }
}
