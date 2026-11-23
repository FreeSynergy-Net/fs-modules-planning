//! Shared by the `FormSource` tests: a real service in a temporary folder, a page of records, a field of a record.
#![allow(dead_code)]

use std::sync::Arc;

use fs_interfaces::common_capnp::cursor;
use fs_interfaces::formsource_capnp::form_source;
use fs_modules_planning::Planning;
use fs_objects::{Store, TursoStore};
use fs_tasks_core::TaskRepo;
use tokio::sync::oneshot;

pub async fn serve(dir: &std::path::Path) -> (std::path::PathBuf, oneshot::Sender<()>) {
    let store = TursoStore::open(dir.join("objects.db")).await.unwrap();
    let store: Arc<dyn Store> = Arc::new(store);
    let repo = Arc::new(TaskRepo::new(store, "https://local.invalid/users/owner"));
    let socket = dir.join("planning.sock");
    let bound = fs_rpc::bind_socket(&socket).await.unwrap();
    let (stop, stopped) = oneshot::channel::<()>();
    tokio::task::spawn_local(async move {
        let _ = fs_rpc::serve_callers(bound, Planning::new(repo, |_| {}), async {
            let _ = stopped.await;
        })
        .await;
    });
    (socket, stop)
}

pub type Rows = Vec<(String, String, Vec<(String, String)>)>;

/// The records of one page: id, and the values by field name.
pub async fn page(
    source: &form_source::Client,
    form: &str,
    scope: &str,
) -> Result<Rows, capnp::Error> {
    let mut request = source.page_request();
    {
        let mut p = request.get();
        p.set_form(form);
        p.set_scope(scope);
        p.set_limit(100);
        let _: cursor::Builder<'_> = p.init_cursor();
    }
    let answer = request.send().promise.await?;
    let records = answer.get()?.get_page()?.get_records()?;
    let mut out = Vec::new();
    for r in records {
        let values = r
            .get_values()?
            .iter()
            .map(|v| Ok((v.get_path()?.to_string()?, v.get_text()?.to_string()?)))
            .collect::<capnp::Result<Vec<_>>>()?;
        out.push((r.get_id()?.to_string()?, String::new(), values));
    }
    Ok(out)
}

pub fn field<'a>(row: &'a (String, String, Vec<(String, String)>), name: &str) -> &'a str {
    row.2
        .iter()
        .find(|(p, _)| p == name)
        .map_or("", |(_, t)| t.as_str())
}
