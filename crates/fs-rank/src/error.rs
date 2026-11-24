//! The error of the adapter.

/// Why a key could not be made or read.
///
/// Pattern: Adapter — the one error type of this crate; the error of the wrapped crate stays inside as text.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    /// The lower neighbour is not smaller than the upper one: there is no place between them.
    #[error("no place between the neighbours: lower key is not smaller than the upper key")]
    NotOrdered,
    /// The text is not a key (lowercase hex, even length, not empty).
    #[error("not a rank key: {0}")]
    BadText(String),
}
