//! `fs-rank`: the hand order of items as a fractional rank.
//!
//! A key is made from the neighbours of the place ([`Key::place`]): between two cards, at the top, at the end. No other
//! card is ever rewritten. Stored as text ([`Key::to_text`]) the keys sort in the database like in memory. Built as a
//! thin adapter around `fracindex` (exact pin, feature `jitter`); the crate can move to its own repo later.
//!
//! Pattern: Facade — this file only declares modules and re-exports the public API.

#![deny(missing_docs)]
#![warn(clippy::pedantic)]
#![allow(clippy::missing_errors_doc, clippy::must_use_candidate)]

mod error;
mod key;
#[cfg(test)]
mod tests;

pub use error::Error;
pub use key::Key;
