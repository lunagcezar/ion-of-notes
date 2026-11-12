//! Capability required by each v2 method and path.
//!
//! The policy table in `policy.rs` is the single source of truth; this module
//! keeps the historical `route_capability` facade.

pub use super::policy::{BlockLimit, MatchKind, RoutePolicy, route_capability, route_policy};
