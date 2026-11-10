//! People, users, bots and partial user references.

use serde::{Deserialize, Serialize};

/// A partial user reference in block metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserRef {
    pub object: String,
    pub id: String,
}

impl UserRef {
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            object: "user".to_string(),
            id: id.into(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UserType {
    Person,
    Bot,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Person {
    #[serde(default)]
    pub email: Option<String>,
    #[serde(default)]
    pub email_verified: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum BotOwner {
    Workspace { workspace: bool },
    User { id: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceLimits {
    pub max_file_upload_size_in_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Bot {
    pub owner: BotOwner,
    #[serde(default)]
    pub workspace_name: Option<String>,
    pub workspace_id: String,
    pub workspace_limits: WorkspaceLimits,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserDto {
    pub object: String,
    pub id: String,
    #[serde(default)]
    pub r#type: Option<UserType>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub avatar_url: Option<String>,
    #[serde(default)]
    pub person: Option<Person>,
    #[serde(default)]
    pub bot: Option<Bot>,
}

impl UserDto {
    pub fn person(
        id: impl Into<String>,
        name: impl Into<String>,
        email: Option<String>,
        include_email: bool,
    ) -> Self {
        Self {
            object: "user".to_string(),
            id: id.into(),
            r#type: Some(UserType::Person),
            name: Some(name.into()),
            avatar_url: None,
            person: Some(Person {
                email: include_email.then_some(email).flatten(),
                email_verified: include_email,
            }),
            bot: None,
        }
    }

    pub fn bot(
        id: impl Into<String>,
        workspace_id: impl Into<String>,
        workspace_name: Option<String>,
    ) -> Self {
        Self {
            object: "user".to_string(),
            id: id.into(),
            r#type: Some(UserType::Bot),
            name: Some("Connection".to_string()),
            avatar_url: None,
            person: None,
            bot: Some(Bot {
                owner: BotOwner::Workspace { workspace: true },
                workspace_name,
                workspace_id: workspace_id.into(),
                workspace_limits: WorkspaceLimits {
                    max_file_upload_size_in_bytes: 5_242_880,
                },
            }),
        }
    }
}
