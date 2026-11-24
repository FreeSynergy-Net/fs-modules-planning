//! The root capability `Tasks` and the factory of the capability of one connection.
//!
//! Pattern: Factory + Adapter — `Planning` builds, for every connection, a root over a view of the store written AS THE CALLER of
//! that connection (the system says who it is: `SO_PEERCRED` on Unix, the pipe client's token on Windows; never a field a client
//! sends). The root hands out `Items` and `Boards` and names what it can do.

use std::rc::Rc;
use std::sync::Arc;

use capnp_rpc::new_client;
use fs_interfaces::tasks_capnp::tasks;
use fs_rpc::{Caller, CallerError, PerConnection};
use fs_tasks_core::TaskRepo;

use crate::boards::BoardsImpl;
use crate::both::both;
use crate::formsource::FormSourceImpl;
use crate::items::ItemsImpl;

/// The contract version this service speaks.
pub const CONTRACT_VERSION: &str = "tasks 0.1 (draft)";

/// The capability ids this service serves (`Items`: create, get, getByUid, delete, list; `Boards`: all five methods).
pub const CAPABILITIES: [&str; 9] = [
    "task.create",
    "task.read",
    "task.delete",
    "board.open",
    "board.create",
    "column.create",
    "column.limit",
    "card.move",
    "board.layout",
];

/// The actor written to the log and to `attributedTo` for a caller: from the system's id of the caller only.
#[must_use]
pub fn actor_of(caller: &Caller) -> String {
    format!("urn:freesynergy:os-user:{}", caller.id())
}

struct Root {
    repo: Arc<TaskRepo>,
}

type Done = Result<(), capnp::Error>;

impl tasks::Server for Root {
    async fn capabilities(
        self: Rc<Self>,
        _: tasks::CapabilitiesParams,
        mut results: tasks::CapabilitiesResults,
    ) -> Done {
        let mut list = results.get().init_capabilities();
        list.set_contract_version(CONTRACT_VERSION);
        let mut ids = list.init_ids(u32::try_from(CAPABILITIES.len()).unwrap_or(0));
        for (i, id) in (0u32..).zip(CAPABILITIES) {
            ids.set(i, id);
        }
        Ok(())
    }

    async fn items(
        self: Rc<Self>,
        _: tasks::ItemsParams,
        mut results: tasks::ItemsResults,
    ) -> Done {
        let items = ItemsImpl {
            items: self.repo.clone(),
        };
        results.get().set_items(new_client(items));
        Ok(())
    }

    async fn boards(
        self: Rc<Self>,
        _: tasks::BoardsParams,
        mut results: tasks::BoardsResults,
    ) -> Done {
        let boards = BoardsImpl {
            boards: self.repo.clone(),
        };
        results.get().set_boards(new_client(boards));
        Ok(())
    }
}

/// The service: one store, one capability per connection.
pub struct Planning {
    repo: Arc<TaskRepo>,
    refused: Rc<dyn Fn(&CallerError)>,
    /// What lives as long as the service (the heartbeat); dropped with it.
    keep: Vec<Box<dyn std::any::Any>>,
}

impl Planning {
    /// Serve `repo`; `refused` is told when the system did not say who is on a connection (that connection is dropped).
    pub fn new(repo: Arc<TaskRepo>, refused: impl Fn(&CallerError) + 'static) -> Self {
        Self {
            repo,
            refused: Rc::new(refused),
            keep: Vec::new(),
        }
    }

    /// Keep `thing` alive as long as the service serves.
    #[must_use]
    pub fn keeping(mut self, thing: impl std::any::Any) -> Self {
        self.keep.push(Box::new(thing));
        self
    }
}

impl PerConnection for Planning {
    fn bootstrap(&mut self, caller: &Caller) -> capnp::capability::Client {
        let repo = Arc::new(self.repo.as_actor(actor_of(caller)));
        let form =
            new_client::<fs_interfaces::formsource_capnp::form_source::Client, _>(FormSourceImpl {
                repo: repo.clone(),
                origin: Rc::default(),
            })
            .client;
        let tasks = new_client::<tasks::Client, _>(Root { repo }).client;
        both(tasks, form)
    }

    fn refused(&mut self, why: &CallerError) {
        (self.refused)(why);
    }
}
