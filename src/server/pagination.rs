//! Cursor pagination for `/api/v2` list endpoints (spec-031).
//!
//! Cursors are opaque to clients: they are a hex-encoded sort key plus offset.
//! The sort key binds a cursor to one endpoint/query so a cursor from another
//! list is rejected instead of silently returning the wrong page.

use crate::server::contract::ListResponse;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PaginationError {
    InvalidPageSize(String),
    InvalidCursor,
}

impl PaginationError {
    pub fn code(&self) -> &'static str {
        match self {
            PaginationError::InvalidPageSize(_) => "validation_error",
            PaginationError::InvalidCursor => "invalid_cursor",
        }
    }

    pub fn message(&self) -> String {
        match self {
            PaginationError::InvalidPageSize(value) => {
                format!("page_size must be between 1 and 100, got {value}")
            }
            PaginationError::InvalidCursor => "invalid or expired cursor".to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageRequest {
    pub start_cursor: Option<String>,
    pub page_size: u16,
}

impl PageRequest {
    pub fn from_query(query: &str) -> Result<Self, PaginationError> {
        let raw_size = super::query_value(query, "page_size");
        let page_size = if raw_size.is_empty() {
            100
        } else {
            raw_size
                .parse::<u16>()
                .map_err(|_| PaginationError::InvalidPageSize(raw_size.clone()))?
        };
        if !(1..=100).contains(&page_size) {
            return Err(PaginationError::InvalidPageSize(raw_size));
        }
        let start_cursor = {
            let value = super::query_value(query, "start_cursor");
            if value.is_empty() { None } else { Some(value) }
        };
        Ok(Self {
            start_cursor,
            page_size,
        })
    }
}

pub fn paginate<T: Clone>(
    items: &[T],
    request: &PageRequest,
    sort: &str,
    type_name: &str,
) -> Result<ListResponse<T>, PaginationError> {
    let offset = match request.start_cursor.as_deref() {
        Some(cursor) => decode_cursor(cursor, sort)?,
        None => 0,
    };
    if offset > items.len() {
        return Err(PaginationError::InvalidCursor);
    }
    let end = (offset + request.page_size as usize).min(items.len());
    let results = items[offset..end].to_vec();
    let has_more = end < items.len();
    let next_cursor = if has_more {
        Some(encode_cursor(sort, end))
    } else {
        None
    };
    Ok(ListResponse::new(type_name, results, has_more, next_cursor))
}

fn encode_cursor(sort: &str, offset: usize) -> String {
    let payload = format!("{sort}\n{offset}");
    to_hex(payload.as_bytes())
}

fn decode_cursor(cursor: &str, expected_sort: &str) -> Result<usize, PaginationError> {
    let bytes = from_hex(cursor).ok_or(PaginationError::InvalidCursor)?;
    let text = String::from_utf8(bytes).map_err(|_| PaginationError::InvalidCursor)?;
    let (sort, offset) = text
        .split_once('\n')
        .ok_or(PaginationError::InvalidCursor)?;
    if sort != expected_sort {
        return Err(PaginationError::InvalidCursor);
    }
    offset
        .parse::<usize>()
        .map_err(|_| PaginationError::InvalidCursor)
}

fn to_hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push_str(&format!("{byte:02x}"));
    }
    out
}

fn from_hex(text: &str) -> Option<Vec<u8>> {
    if !text.len().is_multiple_of(2) {
        return None;
    }
    let mut out = Vec::with_capacity(text.len() / 2);
    let bytes = text.as_bytes();
    for pair in bytes.as_chunks::<2>().0 {
        let hi = (pair[0] as char).to_digit(16)?;
        let lo = (pair[1] as char).to_digit(16)?;
        out.push((hi * 16 + lo) as u8);
    }
    Some(out)
}
