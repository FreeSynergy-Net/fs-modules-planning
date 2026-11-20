//! The contract's messages as plain values, and back.
//!
//! Pattern: Adapter — `Board`, `BoardLayout` and the task list of `tasks.capnp` to values the client keeps (ids as two words, so the
//! crate needs no uuid type), and the request of `editColumns` from a snapshot.

use std::collections::BTreeMap;

use fs_interfaces::common_capnp::uuid;
use fs_interfaces::tasks_capnp::{board, board_layout, boards, task};

use crate::model::{Card, Column};

/// An id of the contract: the two words of `Uuid`.
pub(crate) type Id = (u64, u64);

pub(crate) fn read_id(r: uuid::Reader<'_>) -> Id {
    (r.get_hi(), r.get_lo())
}

pub(crate) fn write_id(mut out: uuid::Builder<'_>, id: Id) {
    out.set_hi(id.0);
    out.set_lo(id.1);
}

/// One column as stored.
#[derive(Clone, Debug)]
pub(crate) struct ColumnData {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) limit: u32,
    pub(crate) progress: Vec<String>,
}

/// A board as read: what `editColumns` needs to replace the columns safely.
#[derive(Clone, Debug)]
pub(crate) struct Snapshot {
    pub(crate) id: Id,
    pub(crate) revision: u64,
    pub(crate) columns: Vec<ColumnData>,
    pub(crate) order: BTreeMap<String, Vec<String>>,
}

fn texts(list: capnp::text_list::Reader<'_>) -> capnp::Result<Vec<String>> {
    list.iter().map(|t| Ok(t?.to_string()?)).collect()
}

pub(crate) fn read_board(r: board::Reader<'_>) -> capnp::Result<Snapshot> {
    let mut columns = Vec::new();
    for c in r.get_columns()? {
        columns.push(ColumnData {
            id: c.get_id()?.to_string()?,
            name: c.get_name()?.to_string()?,
            limit: c.get_wip_limit(),
            progress: texts(c.get_progress()?)?,
        });
    }
    let mut order = BTreeMap::new();
    for p in r.get_order()? {
        order.insert(p.get_column()?.to_string()?, texts(p.get_cards()?)?);
    }
    Ok(Snapshot {
        id: read_id(r.get_envelope()?.get_id()?),
        revision: r.get_revision(),
        columns,
        order,
    })
}

/// The columns with their `uid`s, as the layout says.
pub(crate) fn read_layout(
    r: board_layout::Reader<'_>,
) -> capnp::Result<Vec<(ColumnData, bool, Vec<String>)>> {
    let mut out = Vec::new();
    for c in r.get_columns()? {
        let column = c.get_column()?;
        let data = ColumnData {
            id: column.get_id()?.to_string()?,
            name: column.get_name()?.to_string()?,
            limit: column.get_wip_limit(),
            progress: texts(column.get_progress()?)?,
        };
        out.push((data, c.get_over_limit(), texts(c.get_cards()?)?));
    }
    Ok(out)
}

/// The column of the host from the layout's column and the titles of the cards.
pub(crate) fn column_of(
    data: ColumnData,
    over_limit: bool,
    uids: Vec<String>,
    titles: &BTreeMap<String, String>,
) -> Column {
    Column {
        id: data.id,
        name: data.name,
        limit: (data.limit != 0).then_some(data.limit),
        over_limit,
        cards: uids
            .into_iter()
            .map(|uid| Card {
                title: titles.get(&uid).cloned().unwrap_or_default(),
                uid,
            })
            .collect(),
    }
}

/// `uid` and `title` of a task of an answer (from its JSCalendar text).
pub(crate) fn read_title(t: task::Reader<'_>) -> capnp::Result<Option<(String, String)>> {
    let value: serde_json::Value = serde_json::from_str(t.get_jscalendar()?.to_str()?)
        .map_err(|e| capnp::Error::failed(e.to_string()))?;
    let text = |key: &str| value.get(key).and_then(serde_json::Value::as_str).map(str::to_string);
    Ok(text("uid").map(|uid| (uid, text("title").unwrap_or_default())))
}

/// Fill the request of `editColumns` with this snapshot (the columns and places as they are to be).
pub(crate) fn write_edit(
    mut p: boards::edit_columns_params::Builder<'_>,
    snapshot: &Snapshot,
) -> capnp::Result<()> {
    let count = |n: usize| u32::try_from(n).map_err(|_| capnp::Error::failed("too long".into()));
    write_id(p.reborrow().init_id(), snapshot.id);
    p.set_revision(snapshot.revision);
    let mut columns = p.reborrow().init_columns(count(snapshot.columns.len())?);
    for (i, c) in (0u32..).zip(&snapshot.columns) {
        let mut out = columns.reborrow().get(i);
        out.set_id(c.id.as_str());
        out.set_name(c.name.as_str());
        out.set_wip_limit(c.limit);
        let mut progress = out.init_progress(count(c.progress.len())?);
        for (j, v) in (0u32..).zip(&c.progress) {
            progress.set(j, v.as_str());
        }
    }
    let mut order = p.init_order(count(snapshot.order.len())?);
    for (i, (column, cards)) in (0u32..).zip(&snapshot.order) {
        let mut out = order.reborrow().get(i);
        out.set_column(column.as_str());
        let mut list = out.init_cards(count(cards.len())?);
        for (j, uid) in (0u32..).zip(cards) {
            list.set(j, uid.as_str());
        }
    }
    Ok(())
}
