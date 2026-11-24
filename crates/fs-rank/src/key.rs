//! The rank key.
//!
//! Pattern: Adapter — `Key` wraps `fracindex::Fracindex` (exact pin) behind the small API FreeSynergy needs.

use crate::Error;
use fracindex::Fracindex;

/// A place in a hand-ordered list. Keys sort like their text form ([`Key::to_text`]) and like the keys themselves ([`Ord`]).
///
/// Every key made here carries a random tail (jitter): two writers with the same neighbours get different keys, so
/// both cards keep their place and nobody has to renumber the others.
///
/// Pattern: Adapter — wraps `fracindex::Fracindex` (pinned exactly) behind the small API FreeSynergy needs.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Key(Fracindex);

impl Key {
    /// The key for the first item of an empty list.
    pub fn first() -> Self {
        Self(Fracindex::default())
    }

    /// A key right after `lower` (the end of the list).
    ///
    /// # Panics
    /// Never in practice: one open bound always has room.
    pub fn after(lower: &Key) -> Self {
        Self(build(Fracindex::builder().after(&lower.0)).expect("one open bound always has room"))
    }

    /// A key right before `upper` (the top of the list).
    ///
    /// # Panics
    /// Never in practice: one open bound always has room.
    pub fn before(upper: &Key) -> Self {
        Self(build(Fracindex::builder().before(&upper.0)).expect("one open bound always has room"))
    }

    /// A key between two neighbours.
    pub fn between(lower: &Key, upper: &Key) -> Result<Self, Error> {
        if lower >= upper {
            return Err(Error::NotOrdered);
        }
        build(Fracindex::builder().between(&lower.0, &upper.0))
            .map(Self)
            .ok_or(Error::NotOrdered)
    }

    /// The key for a place described by its neighbours: both given = between, one = at the end or top, none = first.
    pub fn place(lower: Option<&Key>, upper: Option<&Key>) -> Result<Self, Error> {
        match (lower, upper) {
            (Some(a), Some(b)) => Self::between(a, b),
            (Some(a), None) => Ok(Self::after(a)),
            (None, Some(b)) => Ok(Self::before(b)),
            (None, None) => Ok(Self::first()),
        }
    }

    /// The text form for a storage column: lowercase hex of the canonical bytes. Sorting the text sorts the keys.
    pub fn to_text(&self) -> String {
        self.0.to_hex()
    }

    /// Reads a key back from its text form.
    pub fn from_text(text: &str) -> Result<Self, Error> {
        let ok = !text.is_empty()
            && text.len().is_multiple_of(2)
            && text
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b));
        if !ok {
            return Err(Error::BadText(text.to_owned()));
        }
        Fracindex::from_hex(text)
            .map(Self)
            .map_err(|e| Error::BadText(e.to_string()))
    }
}

fn build(builder: fracindex::FracindexBuilder<'_>) -> Option<Fracindex> {
    builder.jitter().build().ok()
}
