//! Pure request-size and workspace-plan limits for `/api/v2` (spec-037).
//!
//! Nothing here touches sockets, files or egui: the HTTP adapter and the
//! server call these checks, and a production deployment can replace the
//! in-process policy with its own configuration.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Maximum JSON body size for one request (500 KB).
pub const MAX_REQUEST_BYTES: usize = 500 * 1024;
/// Maximum number of block elements in one request.
pub const MAX_BLOCK_ELEMENTS: usize = 1000;
/// Maximum characters in one rich-text `text.content`.
pub const MAX_RICH_TEXT_CHARS: usize = 2000;
/// Maximum characters in any URL.
pub const MAX_URL_CHARS: usize = 2000;
/// Maximum characters in one equation expression.
pub const MAX_EQUATION_CHARS: usize = 1000;
/// Maximum items in a rich-text, multi-select, relation or people array.
pub const MAX_ARRAY_ITEMS: usize = 100;
/// Maximum characters in an email address.
pub const MAX_EMAIL_CHARS: usize = 200;
/// Maximum characters in a phone number.
pub const MAX_PHONE_CHARS: usize = 200;
/// Lifetime block limit for a Free workspace with more than one member.
pub const FREE_MULTI_MEMBER_BLOCK_LIMIT: u64 = 1000;

/// Workspace plan, used only for operational limits (not billing).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Plan {
    Free,
    Business,
    Enterprise,
}

impl Plan {
    /// Per-connection request budget per 60-second window.
    pub fn requests_per_minute(self) -> u32 {
        match self {
            Plan::Free => 180,
            Plan::Business | Plan::Enterprise => 600,
        }
    }

    /// Paid plans do not enforce the Free lifetime block limit.
    pub fn has_unlimited_blocks(self) -> bool {
        !matches!(self, Plan::Free)
    }
}

/// The policy inputs derived from the workspace at request time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RequestBudget {
    pub plan: Plan,
    pub members: usize,
    pub blocks: u64,
}

impl RequestBudget {
    pub fn per_connection_limit(&self) -> u32 {
        self.plan.requests_per_minute()
    }

    /// The workspace budget is shared by every connection. The public API does
    /// not publish the exact number, so we scale the per-connection budget.
    pub fn per_workspace_limit(&self) -> u32 {
        self.plan.requests_per_minute().saturating_mul(10)
    }

    /// Whether creating `additional` more blocks would exceed the Free
    /// multi-member workspace limit. The write that reaches the limit succeeds.
    pub fn block_limit_reached(&self, additional: u64) -> bool {
        !self.plan.has_unlimited_blocks()
            && self.members > 1
            && self.blocks.saturating_add(additional) > FREE_MULTI_MEMBER_BLOCK_LIMIT
    }
}

/// One rejected request-size check.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LimitViolation {
    /// Stable identifier copied to `additional_data.limit`.
    pub limit: String,
    pub message: String,
}

impl LimitViolation {
    fn new(limit: &str, message: impl Into<String>) -> Self {
        Self {
            limit: limit.to_string(),
            message: message.into(),
        }
    }
}

/// Validate one JSON request body against the documented size limits.
///
/// Invalid JSON is ignored here: `parse_body` owns the `invalid_json` error so
/// the caller sees one consistent code. The checks are intentionally generic:
/// they inspect the serialized shape, so every current and future DTO inherits
/// the same limits without per-route code.
pub fn validate_request(body: Option<&str>) -> Result<(), LimitViolation> {
    let Some(body) = body else {
        return Ok(());
    };
    if body.len() > MAX_REQUEST_BYTES {
        return Err(LimitViolation::new(
            "request_bytes",
            format!("request body exceeds {MAX_REQUEST_BYTES} bytes"),
        ));
    }
    let Ok(value) = serde_json::from_str::<Value>(body) else {
        return Ok(());
    };
    validate_value(&value)?;
    let blocks = count_blocks(&value, false);
    if blocks > MAX_BLOCK_ELEMENTS {
        return Err(LimitViolation::new(
            "block_elements",
            format!("request contains {blocks} block elements; the limit is {MAX_BLOCK_ELEMENTS}"),
        ));
    }
    Ok(())
}

fn validate_value(value: &Value) -> Result<(), LimitViolation> {
    match value {
        Value::Object(map) => {
            if let Some(url) = map.get("url").and_then(Value::as_str)
                && url.chars().count() > MAX_URL_CHARS
            {
                return Err(LimitViolation::new(
                    "url",
                    format!("URL exceeds {MAX_URL_CHARS} characters"),
                ));
            }
            if let Some(email) = map.get("email").and_then(Value::as_str)
                && email.chars().count() > MAX_EMAIL_CHARS
            {
                return Err(LimitViolation::new(
                    "email",
                    format!("email exceeds {MAX_EMAIL_CHARS} characters"),
                ));
            }
            for key in ["phone_number", "phone"] {
                if let Some(phone) = map.get(key).and_then(Value::as_str)
                    && phone.chars().count() > MAX_PHONE_CHARS
                {
                    return Err(LimitViolation::new(
                        "phone",
                        format!("phone number exceeds {MAX_PHONE_CHARS} characters"),
                    ));
                }
            }
            if let Some(text) = map.get("text").and_then(Value::as_object)
                && let Some(content) = text.get("content").and_then(Value::as_str)
                && content.chars().count() > MAX_RICH_TEXT_CHARS
            {
                return Err(LimitViolation::new(
                    "rich_text_content",
                    format!("rich text content exceeds {MAX_RICH_TEXT_CHARS} characters"),
                ));
            }
            if let Some(equation) = map.get("equation").and_then(Value::as_object)
                && let Some(expression) = equation.get("expression").and_then(Value::as_str)
                && expression.chars().count() > MAX_EQUATION_CHARS
            {
                return Err(LimitViolation::new(
                    "equation",
                    format!("equation exceeds {MAX_EQUATION_CHARS} characters"),
                ));
            }
            for key in ["rich_text", "multi_select", "relation", "people"] {
                if let Some(items) = map.get(key).and_then(Value::as_array)
                    && items.len() > MAX_ARRAY_ITEMS
                {
                    return Err(LimitViolation::new(
                        "array_items",
                        format!(
                            "{key} contains {} items; the limit is {MAX_ARRAY_ITEMS}",
                            items.len()
                        ),
                    ));
                }
            }
            for child in map.values() {
                validate_value(child)?;
            }
        }
        Value::Array(items) => {
            for item in items {
                validate_value(item)?;
            }
        }
        _ => {}
    }
    Ok(())
}

/// Count block elements: every entry of a `children`/`blocks` array is one
/// block, and a standalone `{"object":"block"}` counts once.
fn count_blocks(value: &Value, already_counted: bool) -> usize {
    match value {
        Value::Object(map) => {
            let mut count = usize::from(
                !already_counted && map.get("object").and_then(Value::as_str) == Some("block"),
            );
            for (key, child) in map {
                if key == "children" || key == "blocks" {
                    if let Some(items) = child.as_array() {
                        for item in items {
                            count += 1 + count_blocks(item, true);
                        }
                    }
                } else {
                    count += count_blocks(child, false);
                }
            }
            count
        }
        Value::Array(items) => items.iter().map(|item| count_blocks(item, false)).sum(),
        _ => 0,
    }
}
