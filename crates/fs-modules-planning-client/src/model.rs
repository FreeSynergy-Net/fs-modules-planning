//! What the host sees of a board: columns, cards, the operations, the errors.
//!
//! Pattern: Value objects + one error type — no capnp type leaves the crate, so the host depends on nothing of the contract.

use std::fmt;

/// One card: the task `uid` and the title shown.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Card {
    /// The task's `uid`.
    pub uid: String,
    /// Its title (empty if the task has none).
    pub title: String,
}

/// One column with its cards, top to bottom.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Column {
    /// Stable id inside the board.
    pub id: String,
    /// The name shown.
    pub name: String,
    /// The limit of cards, if any.
    pub limit: Option<u32>,
    /// More cards than the limit says (a report, never a refusal).
    pub over_limit: bool,
    /// The cards.
    pub cards: Vec<Card>,
}

/// What the board component can ask for; `Planning::can` answers from the capabilities the service reported.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Operation {
    /// A new column.
    AddColumn,
    /// A column renamed.
    Rename,
    /// A limit set on a column.
    Limit,
    /// A card moved.
    Move,
    /// A card made.
    AddCard,
}

impl Operation {
    /// The capability id the service must report for this operation.
    #[must_use]
    pub fn capability(self) -> &'static str {
        match self {
            // The contract serves a rename with `editColumns`, as the creation of a column.
            Self::AddColumn | Self::Rename => "column.create",
            Self::Limit => "column.limit",
            Self::Move => "card.move",
            Self::AddCard => "task.create",
        }
    }
}

/// Why a call did not work.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// Nothing listens on the socket (or it is served by another user).
    Unreachable,
    /// The board changed since it was read; nothing was written. Read again.
    Conflict,
    /// The service refused the request as not valid.
    Rejected,
    /// The service does not do this (no such capability).
    Unsupported,
    /// There is no board and none can be made.
    NoBoard,
    /// Anything else the transport or the service said.
    Failed,
}

/// An error of the client: its kind and the words of the service.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Error {
    /// What kind of trouble.
    pub kind: Kind,
    /// The text, for the log and for people.
    pub text: String,
}

impl Error {
    pub(crate) fn new(kind: Kind, text: impl Into<String>) -> Self {
        Self {
            kind,
            text: text.into(),
        }
    }
}

impl From<capnp::Error> for Error {
    fn from(e: capnp::Error) -> Self {
        let text = e.to_string();
        let kind = if text.contains("conflict:") {
            Kind::Conflict
        } else if text.contains("rejected:") {
            Kind::Rejected
        } else if matches!(e.kind, capnp::ErrorKind::Unimplemented) || text.contains("Not possible")
        {
            Kind::Unsupported
        } else {
            Kind::Failed
        };
        Self { kind, text }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}: {}", self.kind, self.text)
    }
}

impl std::error::Error for Error {}
