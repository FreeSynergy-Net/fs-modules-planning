//! The hand order of the cards of a board: one place per card (column and rank key), never a list per column.
//!
//! Pattern: Value object + pure functions over `Board`. A place is a field of the board (a card = one field), so two people who move
//! different cards merge field by field. The key is text made by `fs-rank` (fractional index): a card put between two neighbours gets
//! a key between theirs and no other card is written. Reading sorts by `(key, uid)`; two equal keys are decided by the uid, there is
//! no renumbering. The cards that were never placed stand after the placed ones, by their `progress` (see `Board::layout`).

use std::collections::{BTreeMap, BTreeSet};

use fs_rank::Key;
use serde::{Deserialize, Serialize};

use super::{Board, Layout};

/// Where one card stands: its column and its rank key (text form of `fs_rank::Key`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Place {
    /// The column id.
    pub column: String,
    /// The rank key as text; sorts like the key.
    pub key: String,
}

fn key_of(place: &Place) -> Result<Key, String> {
    Key::from_text(&place.key).map_err(|e| e.to_string())
}

/// The key between two neighbours; if they are equal (two writers, no room between them) after the lower one.
fn between(lower: Option<&Key>, upper: Option<&Key>) -> Key {
    Key::place(lower, upper)
        .unwrap_or_else(|_| Key::after(lower.expect("a failed place has a lower key")))
}

impl Board {
    /// The placed cards per column, top to bottom (sorted by `(key, uid)`); derived from `place`.
    pub fn order(&self) -> BTreeMap<String, Vec<String>> {
        let mut sorted: Vec<(&Place, &String)> = self.place.iter().map(|(u, p)| (p, u)).collect();
        sorted.sort_by(|a, b| (&a.0.key, a.1).cmp(&(&b.0.key, b.1)));
        let mut out: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for (place, uid) in sorted {
            out.entry(place.column.clone())
                .or_default()
                .push(uid.clone());
        }
        out
    }

    /// Replace the places by this order (a card per column, top to bottom). Unchanged if it is the order already stored, so a client that
    /// writes back what it read does not rewrite any key.
    ///
    /// # Errors
    /// A card is placed more than once.
    pub fn set_order(&mut self, order: &BTreeMap<String, Vec<String>>) -> Result<(), String> {
        if *order == self.order() {
            return Ok(());
        }
        let mut seen = BTreeSet::new();
        let mut place = BTreeMap::new();
        for (column, cards) in order {
            let mut last: Option<Key> = None;
            for uid in cards {
                if !seen.insert(uid.as_str()) {
                    return Err(format!("the card {uid:?} is placed more than once"));
                }
                let key = last.as_ref().map_or_else(Key::first, Key::after);
                let text = key.to_text();
                last = Some(key);
                place.insert(
                    uid.clone(),
                    Place {
                        column: column.clone(),
                        key: text,
                    },
                );
            }
        }
        self.place = place;
        Ok(())
    }

    /// Put the card `uid` into `column` at `index` among the cards PLACED in that column (end if `None` or too large), taking it out of
    /// any other place. The progress of the task is not changed, and a full column does not refuse the card.
    ///
    /// # Errors
    /// The column does not exist, or a stored key is not a key.
    pub fn move_card(
        &mut self,
        uid: &str,
        column: &str,
        index: Option<usize>,
    ) -> Result<(), String> {
        self.check_column(column)?;
        self.place.remove(uid);
        let cards = self.order().remove(column).unwrap_or_default();
        let at = index.map_or(cards.len(), |i| i.min(cards.len()));
        let key_at = |i: usize| -> Result<Option<Key>, String> {
            cards.get(i).map(|u| key_of(&self.place[u])).transpose()
        };
        let lower = at.checked_sub(1).map_or(Ok(None), key_at)?;
        let upper = key_at(at)?;
        let key = between(lower.as_ref(), upper.as_ref()).to_text();
        self.place.insert(
            uid.to_owned(),
            Place {
                column: column.to_owned(),
                key,
            },
        );
        Ok(())
    }

    /// Put the card `uid` into `column` between the neighbours it was dropped between, as the `layout` shows the column (the cards that
    /// were never placed count too). `after` is the card it comes right behind, `before` the card it comes right before; `None` = no
    /// card on that side. If something moved in between, `after` is the anchor (right behind it), else `before`, else the end: nothing is
    /// refused and nothing is lost. Cards that stand before the place without a place of their own are given one (where they stand now),
    /// so nothing moves on the screen; the cards behind it keep following their `progress`.
    ///
    /// # Errors
    /// The column does not exist, or a stored key is not a key.
    pub fn place_card(
        &mut self,
        layout: &Layout,
        uid: &str,
        column: &str,
        after: Option<&str>,
        before: Option<&str>,
    ) -> Result<(), String> {
        self.check_column(column)?;
        let shown: Vec<&String> = layout
            .columns
            .iter()
            .find(|c| c.column.id == column)
            .map(|c| c.cards.iter().filter(|u| *u != uid).collect())
            .unwrap_or_default();
        let at = Self::anchor(&shown, after, before);
        self.place
            .retain(|u, _| u == uid || layout.columns.iter().any(|c| c.cards.contains(u)));
        self.place.remove(uid);
        let mut lower: Option<Key> = None;
        for card in &shown[..at] {
            let key = match self.place.get(*card) {
                Some(p) => key_of(p)?,
                None => {
                    let key = between(lower.as_ref(), None);
                    let place = Place {
                        column: column.to_owned(),
                        key: key.to_text(),
                    };
                    self.place.insert((*card).clone(), place);
                    key
                }
            };
            lower = Some(key);
        }
        let upper = shown
            .get(at)
            .and_then(|u| self.place.get(*u))
            .map(key_of)
            .transpose()?;
        let key = between(lower.as_ref(), upper.as_ref()).to_text();
        self.place.insert(
            uid.to_owned(),
            Place {
                column: column.to_owned(),
                key,
            },
        );
        Ok(())
    }

    fn anchor(shown: &[&String], after: Option<&str>, before: Option<&str>) -> usize {
        let pos = |wanted: Option<&str>| wanted.and_then(|w| shown.iter().position(|u| *u == w));
        pos(after)
            .map(|p| p + 1)
            .or_else(|| pos(before))
            .unwrap_or(shown.len())
    }

    fn check_column(&self, column: &str) -> Result<(), String> {
        if self.columns.iter().any(|c| c.id == column) {
            Ok(())
        } else {
            Err(format!("no column {column:?}"))
        }
    }
}
