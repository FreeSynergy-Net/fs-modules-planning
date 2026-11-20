//! `Items` of the planning module: the methods the board needs (create, read, delete, list).
//!
//! Pattern: Adapter — wire request in, `TaskItems` call, wire response out. The other methods of the contract answer
//! `unimplemented` (the capability list does not name them); they are added with the programs that need them.

use std::rc::Rc;
use std::sync::Arc;

use fs_interfaces::tasks_capnp::items;
use fs_rpc::{check_answer, run_to_end};
use fs_tasks_core::TaskItems;

use crate::errors::rpc_error;
use crate::wire::{count, read_id, read_task, required, write_task};

pub(crate) struct ItemsImpl {
    pub(crate) items: Arc<dyn TaskItems>,
}

type Done = Result<(), capnp::Error>;

impl items::Server for ItemsImpl {
    async fn create(
        self: Rc<Self>,
        params: items::CreateParams,
        mut results: items::CreateResults,
    ) -> Done {
        let task = read_task(params.get()?.get_jscalendar()?)?;
        let items = self.items.clone();
        let entry = run_to_end(async move { items.create(task).await })
            .await?
            .map_err(rpc_error)?;
        write_task(results.get().init_task(), &entry);
        Ok(())
    }

    async fn get(
        self: Rc<Self>,
        params: items::GetParams,
        mut results: items::GetResults,
    ) -> Done {
        let id = read_id(params.get()?.get_id()?)?;
        let found = self.items.get(id).await.map_err(rpc_error)?;
        let mut out = results.get();
        out.set_found(found.is_some());
        if let Some(entry) = found {
            write_task(out.init_task(), &entry);
        }
        Ok(())
    }

    async fn get_by_uid(
        self: Rc<Self>,
        params: items::GetByUidParams,
        mut results: items::GetByUidResults,
    ) -> Done {
        let uid = required(params.get()?.get_uid()?, "uid")?.to_string();
        let found = self.items.get_by_uid(&uid).await.map_err(rpc_error)?;
        let mut out = results.get();
        out.set_found(found.is_some());
        if let Some(entry) = found {
            write_task(out.init_task(), &entry);
        }
        Ok(())
    }

    async fn delete(
        self: Rc<Self>,
        params: items::DeleteParams,
        _results: items::DeleteResults,
    ) -> Done {
        let id = read_id(params.get()?.get_id()?)?;
        let items = self.items.clone();
        run_to_end(async move { items.delete(id).await })
            .await?
            .map_err(rpc_error)
    }

    /// Every task in one answer (no cursor yet: a board is small); an answer that would not fit one message is refused.
    async fn list(
        self: Rc<Self>,
        _params: items::ListParams,
        mut results: items::ListResults,
    ) -> Done {
        let entries = self.items.list().await.map_err(rpc_error)?;
        let weight: usize = entries
            .iter()
            .map(|e| e.task.to_value().to_string().len() + 512)
            .sum();
        check_answer(weight)?;
        let mut page = results.get().init_page();
        let mut list = page.reborrow().init_items(count(entries.len())?);
        for (i, entry) in (0u32..).zip(&entries) {
            write_task(list.reborrow().get(i), entry);
        }
        Ok(())
    }
}
