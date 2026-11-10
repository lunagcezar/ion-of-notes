//! Rich text, mentions, annotations and equations.

use serde::{Deserialize, Serialize};

/// Rich-text annotations (spec-032).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Annotations {
    pub bold: bool,
    pub italic: bool,
    pub strikethrough: bool,
    pub underline: bool,
    pub code: bool,
    pub color: String,
}

impl Default for Annotations {
    fn default() -> Self {
        Self {
            bold: false,
            italic: false,
            strikethrough: false,
            underline: false,
            code: false,
            color: "default".to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextLink {
    pub url: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextContent {
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub link: Option<TextLink>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Equation {
    pub expression: String,
}

/// A mention target. This is our v2 shape, not a byte-for-byte Notion clone.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Mention {
    Page { id: String },
    User { id: String },
    Date { start: String, end: Option<String> },
}

/// One rich-text object: text, mention or equation with annotations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RichText {
    Text {
        text: TextContent,
        annotations: Annotations,
        plain_text: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        href: Option<String>,
    },
    Mention {
        mention: Mention,
        annotations: Annotations,
        plain_text: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        href: Option<String>,
    },
    Equation {
        equation: Equation,
        annotations: Annotations,
        plain_text: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        href: Option<String>,
    },
}

impl RichText {
    pub fn text(content: impl Into<String>) -> Self {
        let content = content.into();
        Self::Text {
            text: TextContent {
                content: content.clone(),
                link: None,
            },
            annotations: Annotations::default(),
            plain_text: content,
            href: None,
        }
    }

    pub fn with_link(content: impl Into<String>, url: impl Into<String>) -> Self {
        let content = content.into();
        let url = url.into();
        Self::Text {
            text: TextContent {
                content: content.clone(),
                link: Some(TextLink { url: url.clone() }),
            },
            annotations: Annotations::default(),
            plain_text: content,
            href: Some(url),
        }
    }

    pub fn equation(expression: impl Into<String>) -> Self {
        let expression = expression.into();
        Self::Equation {
            equation: Equation {
                expression: expression.clone(),
            },
            annotations: Annotations::default(),
            plain_text: expression,
            href: None,
        }
    }

    pub fn plain_text(&self) -> &str {
        match self {
            RichText::Text { plain_text, .. }
            | RichText::Mention { plain_text, .. }
            | RichText::Equation { plain_text, .. } => plain_text,
        }
    }

    pub fn set_plain_text(&mut self, value: impl Into<String>) {
        let value = value.into();
        match self {
            RichText::Text {
                text, plain_text, ..
            } => {
                text.content = value.clone();
                *plain_text = value;
            }
            RichText::Mention { plain_text, .. } | RichText::Equation { plain_text, .. } => {
                *plain_text = value;
            }
        }
    }

    pub fn is_plain_text(&self) -> bool {
        matches!(self, RichText::Text { text, .. } if text.link.is_none())
    }
}
