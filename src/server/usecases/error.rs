//! One application error type; each transport adapter maps it explicitly.

/// A use case failed for one of three reasons the adapters can distinguish.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::server) enum UseCaseError {
    /// The request is well formed but violates a rule (`400`).
    Invalid(String),
    /// The addressed entity does not exist (`404`).
    NotFound(String),
    /// Persistence failed after the in-memory change (`500`).
    Storage(String),
}
