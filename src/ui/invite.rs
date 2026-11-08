//! Pure helpers for the Add members / Share dialog (spec-023).

/// Split a comma-separated invite field into trimmed, non-empty entries.
pub fn parse_invite_emails(input: &str) -> Vec<String> {
    input
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .collect()
}

/// A small sanity check before sending an invite to the server.
pub fn is_valid_email(email: &str) -> bool {
    let email = email.trim();
    let Some((local, domain)) = email.split_once('@') else {
        return false;
    };
    !local.is_empty() && domain.contains('.') && !domain.starts_with('.') && !domain.ends_with('.')
}

/// Derive a readable member name from an email local part.
pub fn member_name_from_email(email: &str) -> String {
    let local = email
        .trim()
        .split('@')
        .next()
        .unwrap_or(email)
        .replace(['.', '_', '-'], " ");
    let name = local
        .split_whitespace()
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ");
    if name.is_empty() {
        "Guest".to_string()
    } else {
        name
    }
}
