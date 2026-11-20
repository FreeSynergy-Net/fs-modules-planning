//! `Boards` of the planning module: the contract over the `Boards` trait of the core.
//!
//! Pattern: Adapter — create, list, layout, editColumns (compare-and-set on the revision) and moveCard, each one call of the
//! trait. Capabilities: board.open (list, layout), board.create (create), column.create and column.limit (editColumns),
//! card.move (moveCard).

use std::collections::BTreeMap;
use std::rc::Rc;
use std::sync::Arc;

use fs_interfaces::tasks_capnp::{board, board_column, board_layout, boards, placement};
use fs_rpc::run_to_end;
use fs_tasks_core::model::{Board, Column, Layout};
use fs_tasks_core::{BoardEntry, Boards, Revision};

use crate::errors::{rejected, rpc_error};
use crate::wire::{count, read_id, write_envelope};

pub(crate) struct BoardsImpl {
    pub(crate) boards: Arc<dyn Boards>,
}

type Done = Result<(), capnp::Error>;

fn write_column(mut out: board_column::Builder<'_>, c: &Column) {
    out.set_id(c.id.as_str());
    out.set_name(c.name.as_str());
    out.set_wip_limit(c.wip_limit.unwrap_or(0));
    let mut list = out.init_progress(u32::try_from(c.progress.len()).unwrap_or(0));
    for (i, p) in (0u32..).zip(&c.progress) {
        list.set(i, p.as_str());
    }
}

fn read_column(r: board_column::Reader<'_>) -> capnp::Result<Column> {
    let limit = r.get_wip_limit();
    Ok(Column {
        id: r.get_id()?.to_string()?,
        name: r.get_name()?.to_string()?,
        wip_limit: (limit != 0).then_some(limit),
        progress: r
            .get_progress()?
            .iter()
            .map(|p| Ok(p?.to_string()?))
            .collect::<capnp::Result<_>>()?,
    })
}

fn write_cards(out: capnp::text_list::Builder<'_>, cards: &[String]) {
    let mut out = out;
    for (i, uid) in (0u32..).zip(cards) {
        out.set(i, uid.as_str());
    }
}

fn write_board(mut out: board::Builder<'_>, entry: &BoardEntry) -> capnp::Result<()> {
    write_envelope(out.reborrow().init_envelope(), &entry.envelope);
    out.set_name(entry.board.name.as_str());
    out.set_label(entry.board.label.as_deref().unwrap_or(""));
    out.set_revision(entry.revision().get());
    let mut columns = out.reborrow().init_columns(count(entry.board.columns.len())?);
    for (i, c) in (0u32..).zip(&entry.board.columns) {
        write_column(columns.reborrow().get(i), c);
    }
    let mut order = out.init_order(count(entry.board.order.len())?);
    for (i, (column, cards)) in (0u32..).zip(&entry.board.order) {
        let mut p = order.reborrow().get(i);
        p.set_column(column.as_str());
        write_cards(p.init_cards(count(cards.len())?), cards);
    }
    Ok(())
}

fn write_layout(out: board_layout::Builder<'_>, layout: &Layout) -> capnp::Result<()> {
    let mut columns = out.init_columns(count(layout.columns.len())?);
    for (i, c) in (0u32..).zip(&layout.columns) {
        let mut out = columns.reborrow().get(i);
        write_column(out.reborrow().init_column(), &c.column);
        write_cards(out.reborrow().init_cards(count(c.cards.len())?), &c.cards);
        out.set_over_limit(c.over_limit);
    }
    Ok(())
}

fn read_order(
    list: capnp::struct_list::Reader<'_, placement::Owned>,
) -> capnp::Result<BTreeMap<String, Vec<String>>> {
    let mut order = BTreeMap::new();
    for p in list {
        let cards = p
            .get_cards()?
            .iter()
            .map(|c| Ok(c?.to_string()?))
            .collect::<capnp::Result<Vec<String>>>()?;
        if order.insert(p.get_column()?.to_string()?, cards).is_some() {
            return Err(rejected("a column is placed twice"));
        }
    }
    Ok(order)
}

impl boards::Server for BoardsImpl {
    async fn create(
        self: Rc<Self>,
        params: boards::CreateParams,
        mut results: boards::CreateResults,
    ) -> Done {
        let p = params.get()?;
        let name = p.get_name()?.to_string()?;
        let label = p.get_label()?.to_string()?;
        let label = (!label.is_empty()).then_some(label);
        let repo = self.boards.clone();
        let entry = run_to_end(async move { repo.board_create(&name, label.as_deref()).await })
            .await?
            .map_err(rpc_error)?;
        write_board(results.get().init_board(), &entry)
    }

    async fn list(
        self: Rc<Self>,
        _params: boards::ListParams,
        mut results: boards::ListResults,
    ) -> Done {
        let entries = self.boards.boards().await.map_err(rpc_error)?;
        let mut list = results.get().init_boards(count(entries.len())?);
        for (i, entry) in (0u32..).zip(&entries) {
            write_board(list.reborrow().get(i), entry)?;
        }
        Ok(())
    }

    async fn layout(
        self: Rc<Self>,
        params: boards::LayoutParams,
        mut results: boards::LayoutResults,
    ) -> Done {
        let id = read_id(params.get()?.get_id()?)?;
        let layout = self.boards.board_layout(id).await.map_err(rpc_error)?;
        write_layout(results.get().init_layout(), &layout)
    }

    async fn edit_columns(
        self: Rc<Self>,
        params: boards::EditColumnsParams,
        mut results: boards::EditColumnsResults,
    ) -> Done {
        let p = params.get()?;
        let id = read_id(p.get_id()?)?;
        let revision = Revision::from(p.get_revision());
        let columns: Vec<Column> = p
            .get_columns()?
            .iter()
            .map(read_column)
            .collect::<capnp::Result<_>>()?;
        let order = read_order(p.get_order()?)?;
        let repo = self.boards.clone();
        let entry = run_to_end(async move {
            // The name and the label stay as stored; only columns and places are replaced here.
            let Some(current) = repo.board_get(id).await? else {
                return Err(fs_tasks_core::Error::Rejected("no such board".into()));
            };
            let board = Board {
                columns,
                order,
                ..current.board
            };
            repo.board_edit(id, revision, board).await
        })
        .await?
        .map_err(rpc_error)?;
        write_board(results.get().init_board(), &entry)
    }

    async fn move_card(
        self: Rc<Self>,
        params: boards::MoveCardParams,
        mut results: boards::MoveCardResults,
    ) -> Done {
        let p = params.get()?;
        let id = read_id(p.get_id()?)?;
        let uid = p.get_uid()?.to_string()?;
        let column = p.get_column()?.to_string()?;
        let index = usize::try_from(p.get_index()).ok();
        let repo = self.boards.clone();
        let entry =
            run_to_end(async move { repo.board_move_card(id, &uid, &column, index).await })
                .await?
                .map_err(rpc_error)?;
        write_board(results.get().init_board(), &entry)
    }
}
