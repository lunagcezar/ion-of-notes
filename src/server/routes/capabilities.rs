//! Capability required by each v2 method and path.

use super::super::{Capability, Method};

pub fn route_capability(method: Method, path: &str) -> Option<Capability> {
    match (method, path) {
        (Method::Get, "/api/v2/workspace") => Some(Capability::ReadContent),
        (Method::Put, "/api/v2/workspace") => Some(Capability::UpdateContent),
        (Method::Get, p) if p.starts_with("/api/v2/blocks/") => Some(Capability::ReadContent),
        (Method::Patch | Method::Delete, p) if p.starts_with("/api/v2/blocks/") => {
            Some(Capability::UpdateContent)
        }
        (Method::Get, "/api/v2/databases") => Some(Capability::ReadContent),
        (Method::Post, "/api/v2/databases") => Some(Capability::InsertContent),
        (Method::Get, p) if p.starts_with("/api/v2/databases/") => Some(Capability::ReadContent),
        (Method::Patch | Method::Delete, p) if p.starts_with("/api/v2/databases/") => {
            Some(Capability::UpdateContent)
        }
        (Method::Get, "/api/v2/auth/session") => Some(Capability::ReadContent),
        (Method::Post, "/api/v2/auth/logout") => Some(Capability::ReadContent),
        (Method::Post, "/api/v2/crdt/sync") => Some(Capability::ReadContent),
        (Method::Post, "/api/v2/crdt/update") => Some(Capability::UpdateContent),
        (Method::Get, "/api/v2/webhooks") => Some(Capability::ReadContent),
        (Method::Post, "/api/v2/webhooks") => Some(Capability::InsertContent),
        (Method::Get, "/api/v2/webhook_deliveries") => Some(Capability::ReadContent),
        (Method::Post, "/api/v2/webhook_deliveries/retry") => Some(Capability::InsertContent),
        (Method::Get, "/api/v2/webhook_deliveries/dead_letters") => Some(Capability::ReadContent),
        (Method::Post, "/api/v2/webhook_deliveries/requeue") => Some(Capability::InsertContent),
        (Method::Delete, p) if p.starts_with("/api/v2/webhooks/") => {
            Some(Capability::UpdateContent)
        }
        (Method::Post, p) if p.starts_with("/api/v2/webhooks/") => Some(Capability::InsertContent),
        (Method::Get, "/api/v2/comments") => Some(Capability::ReadComments),
        (Method::Post, "/api/v2/comments") => Some(Capability::InsertComments),
        (Method::Get, p) if p.starts_with("/api/v2/comments/") => Some(Capability::ReadComments),
        (Method::Patch | Method::Delete, p) if p.starts_with("/api/v2/comments/") => {
            Some(Capability::InsertComments)
        }
        (Method::Get, p) if p.starts_with("/api/v2/users") => Some(Capability::ReadContent),
        (Method::Get, "/api/v2/file_uploads") => Some(Capability::ReadContent),
        (Method::Post, "/api/v2/file_uploads") => Some(Capability::InsertContent),
        (Method::Get, p) if p.starts_with("/api/v2/file_uploads/") => Some(Capability::ReadContent),
        (Method::Post, p) if p.starts_with("/api/v2/file_uploads/") => {
            Some(Capability::InsertContent)
        }
        (Method::Get, "/api/v2/views") => Some(Capability::ReadContent),
        (Method::Post, "/api/v2/views") => Some(Capability::InsertContent),
        (Method::Get, p) if p.starts_with("/api/v2/views/") => Some(Capability::ReadContent),
        (Method::Patch | Method::Delete, p) if p.starts_with("/api/v2/views/") => {
            Some(Capability::UpdateContent)
        }
        (Method::Post, p) if p.starts_with("/api/v2/views/") => Some(Capability::ReadContent),
        (Method::Get, p) if p.starts_with("/api/v2/data_sources/") => Some(Capability::ReadContent),
        (Method::Patch, p) if p.starts_with("/api/v2/data_sources/") => {
            Some(Capability::UpdateContent)
        }
        (Method::Post, p) if p.ends_with("/query") && p.starts_with("/api/v2/data_sources/") => {
            Some(Capability::ReadContent)
        }
        (Method::Post, p)
            if p.ends_with("/data_sources") && p.starts_with("/api/v2/databases/") =>
        {
            Some(Capability::InsertContent)
        }
        (Method::Get, "/api/v2/pages") => Some(Capability::ReadContent),
        (Method::Post, "/api/v2/pages") => Some(Capability::InsertContent),
        (Method::Get, p) if p.starts_with("/api/v2/pages/") => Some(Capability::ReadContent),
        (Method::Patch | Method::Delete, p) if p.starts_with("/api/v2/pages/") => {
            Some(Capability::UpdateContent)
        }
        (Method::Get, "/api/v2/search") => Some(Capability::ReadContent),
        (Method::Get, "/api/v2/members") => Some(Capability::ReadContent),
        (Method::Post, "/api/v2/members") => Some(Capability::InsertContent),
        (Method::Get, "/api/v2/shares") => Some(Capability::ReadContent),
        (Method::Put, "/api/v2/shares") => Some(Capability::UpdateContent),
        (Method::Get, "/api/v2/sync") => Some(Capability::ReadContent),
        (Method::Get, "/api/v2/export") => Some(Capability::ReadContent),
        (Method::Get, "/api/v2/realtime") => Some(Capability::ReadContent),
        _ => None,
    }
}
