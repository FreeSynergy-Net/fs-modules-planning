//! Column helpers of the board edits.
//!
//! Pattern: Facade — the small pure functions `Planning` uses to change the columns of a [`Snapshot`] (find a column, make an id).

use crate::model::{Error, Kind};
use crate::wire::{ColumnData, Snapshot};

/// The column `id` of the snapshot; an unknown column is [`Kind::Rejected`].
pub(crate) fn column<'a>(snapshot: &'a mut Snapshot, id: &str) -> Result<&'a mut ColumnData, Error> {
    snapshot
        .columns
        .iter_mut()
        .find(|c| c.id == id)
        .ok_or_else(|| Error::new(Kind::Rejected, format!("no column {id:?}")))
}

/// An id for a new column: the name in lower-case letters and digits with hyphens, made unique with a number.
pub(crate) fn unused_id(name: &str, columns: &[ColumnData]) -> String {
    let mut slug: String = name
        .chars()
        .map(|c| if c.is_alphanumeric() { c.to_ascii_lowercase() } else { '-' })
        .collect();
    slug = slug.split('-').filter(|p| !p.is_empty()).collect::<Vec<_>>().join("-");
    if slug.is_empty() {
        slug = "column".into();
    }
    let taken = |id: &str| columns.iter().any(|c| c.id == id);
    let mut id = slug.clone();
    let mut n = 2;
    while taken(&id) {
        id = format!("{slug}-{n}");
        n += 1;
    }
    id
}
