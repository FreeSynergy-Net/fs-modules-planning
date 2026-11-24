//! Validation of a Task against JSCalendar 2.0: module map.
//!
//! Pattern: Facade (declarations) — callers use `validate::{issues, validate}`.

mod check;
mod color;
mod css_names;

pub use check::{PROGRESS, issues, validate};
