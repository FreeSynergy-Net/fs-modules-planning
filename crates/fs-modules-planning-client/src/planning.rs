//! `Planning`: the board of the planning module as the host needs it.
//!
//! Pattern: Adapter (the Records and Events side of the board component over the contract `tasks.capnp`) + Proxy. Every call is one
//! call of the catalog (`catalog/commands.json`); `can` answers from the capabilities the service reported at the connect, so a
//! control is only unlocked where the service says it can. The host links this crate (K2/K3 transition; a host that loads modules
//! at run time replaces that, ADR 0104). Everything runs on a `tokio::task::LocalSet` (capnp-rpc).

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use fs_interfaces::tasks_capnp::{boards, items, tasks};
use fs_rpc::Connection;

use crate::catalog::Catalog;
use crate::columns::{column, unused_id};
use crate::model::{Card, Column, Error, Kind, Operation};
use crate::wire::{
    ColumnData, Id, Snapshot, column_of, read_board, read_layout, read_title, write_edit, write_id,
};

/// The name the first board gets when the store has none.
const FIRST_BOARD: &str = "Tasks";

/// A connection to the planning service and the board it shows.
pub struct Planning {
    tasks: tasks::Client,
    caps: BTreeSet<String>,
    catalog: Catalog,
    board: Option<Id>,
    /// Kept for its `Drop`: the connection ends with the client.
    #[allow(dead_code)]
    connection: Option<Connection<tasks::Client>>,
}

impl Planning {
    /// Connect to the service on `socket` (reach it with [`ensure_service`](crate::ensure_service) first).
    ///
    /// # Errors
    /// [`Kind::Unreachable`]: nothing listens there or it is served by another user; otherwise what the service said.
    pub async fn connect(socket: &Path) -> Result<Self, Error> {
        let connection = fs_rpc::connect::<tasks::Client>(socket)
            .await
            .map_err(|e| Error::new(Kind::Unreachable, format!("{}: {e}", socket.display())))?;
        let mut planning = Self::over(connection.client.clone()).await?;
        planning.connection = Some(connection);
        Ok(planning)
    }

    /// Use a root capability that is already there (a test, or a host that connected itself).
    ///
    /// # Errors
    /// The service did not answer `capabilities`.
    pub async fn over(tasks: tasks::Client) -> Result<Self, Error> {
        let response = tasks.capabilities_request().send().promise.await?;
        let caps = response
            .get()?
            .get_capabilities()?
            .get_ids()?
            .iter()
            .map(|id| Ok(id?.to_string()?))
            .collect::<capnp::Result<_>>()?;
        Ok(Self {
            tasks,
            caps,
            catalog: Catalog::bundled(),
            board: None,
            connection: None,
        })
    }

    /// Whether the service reported what `operation` needs.
    #[must_use]
    pub fn can(&self, operation: Operation) -> bool {
        self.caps.contains(operation.capability())
    }

    /// Whether the command of the catalog can run: the catalog has it and the service reported its capability.
    #[must_use]
    pub fn can_command(&self, id: &str) -> bool {
        self.catalog
            .command(id)
            .is_some_and(|c| self.caps.contains(c.contract_capability()))
    }

    /// The catalog this client runs commands from.
    #[must_use]
    pub fn catalog(&self) -> &Catalog {
        &self.catalog
    }

    fn boards(&self) -> boards::Client {
        self.tasks.boards_request().send().pipeline.get_boards()
    }

    fn items(&self) -> items::Client {
        self.tasks.items_request().send().pipeline.get_items()
    }

    fn need(&self, operation: Operation) -> Result<(), Error> {
        if self.can(operation) {
            Ok(())
        } else {
            Err(Error::new(
                Kind::Unsupported,
                format!("the service does not report {}", operation.capability()),
            ))
        }
    }

    /// The board shown: the first board by name; one is made when the store has none and the service can.
    async fn board_id(&mut self) -> Result<Id, Error> {
        if let Some(id) = self.board {
            return Ok(id);
        }
        let boards = self.boards();
        let response = boards.list_request().send().promise.await?;
        let first = response.get()?.get_boards()?.iter().next().map(read_board);
        let id = match first {
            Some(snapshot) => snapshot?.id,
            None if self.caps.contains("board.create") => {
                let mut request = boards.create_request();
                request.get().set_name(FIRST_BOARD);
                let response = request.send().promise.await?;
                read_board(response.get()?.get_board()?)?.id
            }
            None => return Err(Error::new(Kind::NoBoard, "no board, and none can be made")),
        };
        self.board = Some(id);
        Ok(id)
    }

    async fn snapshot(&mut self) -> Result<Snapshot, Error> {
        let id = self.board_id().await?;
        let response = self.boards().list_request().send().promise.await?;
        for board in response.get()?.get_boards()? {
            let snapshot = read_board(board)?;
            if snapshot.id == id {
                return Ok(snapshot);
            }
        }
        Err(Error::new(Kind::NoBoard, "the board is gone"))
    }

    /// The columns with their cards now (the titles come from the tasks).
    ///
    /// # Errors
    /// No board, or what the service said.
    pub async fn columns(&mut self) -> Result<Vec<Column>, Error> {
        let id = self.board_id().await?;
        let mut request = self.boards().layout_request();
        write_id(request.get().init_id(), id);
        let response = request.send().promise.await?;
        let layout = read_layout(response.get()?.get_layout()?)?;
        let tasks = self.items().list_request().send().promise.await?;
        let mut titles = BTreeMap::new();
        for task in tasks.get()?.get_page()?.get_items()? {
            if let Some((uid, title)) = read_title(task)? {
                titles.insert(uid, title);
            }
        }
        Ok(layout
            .into_iter()
            .map(|(data, over, uids)| column_of(data, over, uids, &titles))
            .collect())
    }

    /// A new card with `title` at the end of the column `column_id` (command `create`, then `move`).
    ///
    /// # Errors
    /// The service does not report `task.create`/`card.move`, the column does not exist, or what the service said.
    pub async fn add_card(&mut self, column_id: &str, title: &str) -> Result<Card, Error> {
        self.need(Operation::AddCard)?;
        self.need(Operation::Move)?;
        let uid = fs_jscalendar::new_uid();
        let now = fs_jscalendar::now_utc();
        let task = serde_json::json!({
            "@type": "Task", "uid": uid, "title": title, "progress": "needs-action",
            "created": now, "updated": now,
        });
        let mut request = self.items().create_request();
        request.get().set_jscalendar(task.to_string().as_str());
        request.send().promise.await?;
        self.move_card(&uid, column_id, None).await?;
        Ok(Card {
            uid,
            title: title.to_string(),
        })
    }

    /// Put the card `uid` in the column `column_id` at `index` (end if `None`; command `move`).
    ///
    /// # Errors
    /// The service does not report `card.move`, the column does not exist, or what the service said.
    pub async fn move_card(
        &mut self,
        uid: &str,
        column_id: &str,
        index: Option<usize>,
    ) -> Result<(), Error> {
        self.need(Operation::Move)?;
        let id = self.board_id().await?;
        let mut request = self.boards().move_card_request();
        let mut p = request.get();
        write_id(p.reborrow().init_id(), id);
        p.set_uid(uid);
        p.set_column(column_id);
        p.set_index(index.map_or(-1, |i| i32::try_from(i).unwrap_or(i32::MAX)));
        request.send().promise.await?;
        Ok(())
    }

    /// Replace the columns of the board by what `change` makes of them (one compare-and-set on the revision just read).
    async fn edit(
        &mut self,
        change: impl FnOnce(&mut Snapshot) -> Result<(), Error>,
    ) -> Result<(), Error> {
        let mut snapshot = self.snapshot().await?;
        change(&mut snapshot)?;
        let mut request = self.boards().edit_columns_request();
        write_edit(request.get(), &snapshot)?;
        request.send().promise.await?;
        Ok(())
    }

    /// A new column named `name` at the right end (command `column.create`); its id is made from the name.
    ///
    /// # Errors
    /// The service does not report `column.create`, the name is empty, the board changed meanwhile ([`Kind::Conflict`]), or what the service said.
    pub async fn add_column(&mut self, name: &str) -> Result<String, Error> {
        self.need(Operation::AddColumn)?;
        let name = name.trim().to_string();
        let mut made = String::new();
        self.edit(|s| {
            if name.is_empty() {
                return Err(Error::new(Kind::Rejected, "a column needs a name"));
            }
            made = unused_id(&name, &s.columns);
            s.columns.push(ColumnData {
                id: made.clone(),
                name,
                limit: 0,
                progress: Vec::new(),
            });
            Ok(())
        })
        .await?;
        Ok(made)
    }

    /// Rename the column `column_id`.
    ///
    /// # Errors
    /// As for [`add_column`](Self::add_column); an unknown column is [`Kind::Rejected`].
    pub async fn rename_column(&mut self, column_id: &str, name: &str) -> Result<(), Error> {
        self.need(Operation::Rename)?;
        self.edit(|s| column(s, column_id).map(|c| c.name = name.trim().to_string()))
            .await
    }

    /// Set (`Some`) or take away (`None`) the card limit of the column `column_id`.
    ///
    /// # Errors
    /// The service does not report `column.limit`, an unknown column is [`Kind::Rejected`], or what the service said.
    pub async fn set_limit(&mut self, column_id: &str, limit: Option<u32>) -> Result<(), Error> {
        self.need(Operation::Limit)?;
        self.edit(|s| column(s, column_id).map(|c| c.limit = limit.unwrap_or(0)))
            .await
    }
}
