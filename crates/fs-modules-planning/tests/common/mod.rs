//! Shared by the `FormSource` tests: a real service in a temporary folder, a page of records, a field of a record.
#![allow(dead_code)]

use std::sync::Arc;

use fs_interfaces::common_capnp::cursor;
use fs_interfaces::formsource_capnp::form_source;
use fs_interfaces::owner::LocalOwner as _;
use fs_interfaces::owner::fake::FixedOwner;
use fs_modules_planning::Planning;
use fs_objects::{Store, TursoStore};
use fs_tasks_core::TaskRepo;
use tokio::sync::oneshot;

pub async fn serve(dir: &std::path::Path) -> (std::path::PathBuf, oneshot::Sender<()>) {
    let (socket, stop, _) = serve_repo(dir).await;
    (socket, stop)
}

/// Like [`serve`], and the repository the service runs on (for what the contract cannot write, e.g. the stored lanes of a board).
pub async fn serve_repo(
    dir: &std::path::Path,
) -> (std::path::PathBuf, oneshot::Sender<()>, Arc<TaskRepo>) {
    // the owner of the store comes from the source of the owner, as in `serve`
    let owner = FixedOwner(fs_objects::testing::owner()).owner().unwrap();
    let options = fs_objects::OpenOptions::default().with_owner(owner.clone());
    let store = TursoStore::open_with(dir.join("objects.db"), options)
        .await
        .unwrap();
    let store: Arc<dyn Store> = Arc::new(store);
    let repo = Arc::new(TaskRepo::new(
        store,
        "https://local.invalid/users/owner",
        owner,
    ));
    let socket = dir.join("planning.sock");
    let bound = fs_rpc::bind_socket(&socket).await.unwrap();
    let (stop, stopped) = oneshot::channel::<()>();
    let service = repo.clone();
    tokio::task::spawn_local(async move {
        let _ = fs_rpc::serve_callers(bound, Planning::new(service, |_| {}), async {
            let _ = stopped.await;
        })
        .await;
    });
    (socket, stop, repo)
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

/// The answer of one `invoke`: what failed (the type and the detail of the problem) or what the command made.
#[derive(Debug, PartialEq, Eq)]
pub struct Answer {
    pub failed: bool,
    pub kind: String,
    pub detail: String,
    pub record: String,
    pub text: String,
}

/// One `invoke` by the capability key, every value a text.
pub async fn invoke(
    source: &form_source::Client,
    command: &str,
    record: &str,
    pairs: &[(&str, &str)],
) -> Answer {
    let mut request = source.invoke_request();
    {
        let mut p = request.get();
        p.set_command(command);
        p.set_record(record);
        let mut args = p.init_args(u32::try_from(pairs.len()).unwrap());
        for (i, (k, v)) in pairs.iter().enumerate() {
            let mut a = args.reborrow().get(u32::try_from(i).unwrap());
            a.set_path(k);
            a.set_text(v);
        }
    }
    let answer = request.send().promise.await.unwrap();
    let answer = answer.get().unwrap();
    let text = |r: capnp::Result<capnp::text::Reader<'_>>| r.unwrap().to_string().unwrap();
    let problem = answer.get_problem().unwrap();
    let result = answer.get_result().unwrap();
    Answer {
        failed: answer.get_failed(),
        kind: text(problem.get_type()),
        detail: text(problem.get_detail()),
        record: text(result.get_record()),
        text: text(result.get_text()),
    }
}

/// `Ok(record)` when the command ran, else the problem type and its detail.
pub async fn call(
    source: &form_source::Client,
    command: &str,
    record: &str,
    pairs: &[(&str, &str)],
) -> Result<String, (String, String)> {
    let a = invoke(source, command, record, pairs).await;
    if a.failed {
        Err((a.kind, a.detail))
    } else {
        Ok(a.record)
    }
}

/// A board `work` with the given titles (standard columns, the cards in `todo` in that order); the uids of the cards.
pub async fn board_with(
    socket: &std::path::Path,
    source: &form_source::Client,
    titles: &[&str],
) -> Vec<String> {
    use fs_interfaces::tasks_capnp::tasks;
    use fs_tasks_core::Task;
    let tasks = fs_rpc::connect::<tasks::Client>(socket).await.unwrap();
    let boards = tasks.client.boards_request().send().pipeline.get_boards();
    let mut create = boards.create_request();
    create.get().set_name("work");
    create.send().promise.await.unwrap();
    let items = tasks.client.items_request().send().pipeline.get_items();
    for title in titles {
        let mut add = items.create_request();
        add.get()
            .set_jscalendar(Task::new(*title).to_value().to_string().as_str());
        add.send().promise.await.unwrap();
    }
    let rows = page(source, "cards-in-columns@1.1", "work").await.unwrap();
    rows.into_iter().map(|r| r.0).collect()
}

/// The titles of the page of `work`, column by column (the order of the layout), as `column/title`.
pub async fn titles(source: &form_source::Client) -> Vec<String> {
    let rows = page(source, "cards-in-columns@1.1", "work").await.unwrap();
    rows.iter()
        .map(|r| format!("{}/{}", field(r, "column"), field(r, "title")))
        .collect()
}

/// Seed the own channels of `cards-in-columns@1.3.0` on the board `work`: a limit of 3 on its column `todo`, and one task
/// `full` with `priority` 1, `color` `#ff8800`, `estimated` `PT1H30M`, a `description`, the zone `Europe/Berlin` and two people in charge.
pub async fn seed_channels(socket: &std::path::Path) {
    use fs_interfaces::tasks_capnp::tasks;
    use fs_tasks_core::Task;
    let tasks = fs_rpc::connect::<tasks::Client>(socket).await.unwrap();
    let boards = tasks.client.boards_request().send().pipeline.get_boards();
    let list = boards.list_request().send().promise.await.unwrap();
    let list = list.get().unwrap().get_boards().unwrap();
    let board = list.get(0);
    let id = board.get_envelope().unwrap().get_id().unwrap();
    let mut edit = boards.edit_columns_request();
    {
        let mut p = edit.get();
        let mut wire = p.reborrow().init_id();
        wire.set_hi(id.get_hi());
        wire.set_lo(id.get_lo());
        p.set_revision(board.get_revision());
        let old = board.get_columns().unwrap();
        let mut columns = p.init_columns(old.len());
        for (i, c) in old.iter().enumerate() {
            let mut out = columns.reborrow().get(u32::try_from(i).unwrap());
            let id = c.get_id().unwrap().to_str().unwrap();
            out.set_id(id);
            out.set_name(c.get_name().unwrap().to_str().unwrap());
            out.set_wip_limit(if id == "todo" { 3 } else { 0 });
            let mut progress = out.init_progress(c.get_progress().unwrap().len());
            for (j, w) in c.get_progress().unwrap().iter().enumerate() {
                progress.set(u32::try_from(j).unwrap(), w.unwrap().to_str().unwrap());
            }
        }
    }
    edit.send().promise.await.unwrap();
    let mut full = Task::new("full");
    full.priority = Some(1);
    full.description = Some("why".into());
    full.estimated_duration = Some("PT1H30M".into());
    full.set_vendor("color", serde_json::json!("#ff8800"));
    full.time_zone = Some("Europe/Berlin".into());
    full.assign_actor(
        "freesynergy.net:person:ada",
        fs_tasks_core::model::ActorType::Person,
        "Ada",
    );
    full.assign_actor(
        "freesynergy.net:person:bob",
        fs_tasks_core::model::ActorType::Person,
        "Bob",
    );
    let items = tasks.client.items_request().send().pipeline.get_items();
    let mut add = items.create_request();
    add.get()
        .set_jscalendar(full.to_value().to_string().as_str());
    add.send().promise.await.unwrap();
}
