//! `FormSource` next to `Tasks` on one socket, over a real service in a temporary folder: hello, a page of cards by column, and the
//! first write: `move` through `invoke` with nothing but text values.
#![cfg(unix)]

use std::sync::Arc;

use fs_interfaces::common_capnp::cursor;
use fs_interfaces::formsource_capnp::form_source;
use fs_interfaces::tasks_capnp::tasks;
use fs_modules_planning::{FORM_CARDS, PROVIDER, Planning};
use fs_objects::{Store, TursoStore};
use fs_tasks_core::{Task, TaskRepo};
use tokio::sync::oneshot;
use tokio::task::LocalSet;

async fn serve(dir: &std::path::Path) -> (std::path::PathBuf, oneshot::Sender<()>) {
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

type Rows = Vec<(String, String, Vec<(String, String)>)>;

/// The records of one page: id, and the values by field name.
async fn page(source: &form_source::Client, form: &str, scope: &str) -> Result<Rows, capnp::Error> {
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

fn field<'a>(row: &'a (String, String, Vec<(String, String)>), name: &str) -> &'a str {
    row.2
        .iter()
        .find(|(p, _)| p == name)
        .map_or("", |(_, t)| t.as_str())
}

#[tokio::test(flavor = "current_thread")]
async fn a_card_is_moved_through_invoke_with_text_values_only() {
    let dir = tempfile::Builder::new().prefix("fs").tempdir().unwrap();
    LocalSet::new()
        .run_until(async {
            let (socket, _stop) = serve(dir.path()).await;
            let source = fs_rpc::connect::<form_source::Client>(&socket)
                .await
                .unwrap();
            let tasks = fs_rpc::connect::<tasks::Client>(&socket).await.unwrap();

            let hello = source.client.hello_request().send().promise.await.unwrap();
            let hello = hello.get().unwrap();
            assert_eq!(hello.get_level(), fs_interfaces::forms::FORM_SOURCE_LEVEL);
            assert_eq!(hello.get_provider().unwrap().to_str().unwrap(), PROVIDER);

            // the other interface answers on the same socket
            let boards = tasks.client.boards_request().send().pipeline.get_boards();
            let mut create = boards.create_request();
            create.get().set_name("work");
            create.send().promise.await.unwrap();
            let items = tasks.client.items_request().send().pipeline.get_items();
            let mut add = items.create_request();
            add.get()
                .set_jscalendar(Task::new("Write the test").to_value().to_string().as_str());
            add.send().promise.await.unwrap();

            let rows = page(&source.client, FORM_CARDS, "work").await.unwrap();
            assert_eq!(rows.len(), 1);
            assert_eq!(field(&rows[0], "title"), "Write the test");
            assert_eq!(field(&rows[0], "column"), "todo");
            let uid = rows[0].0.clone();

            // every field the catalog names is one the service writes (`due` only for a task that has one)
            let forms: serde_json::Value =
                serde_json::from_str(include_str!("../../../catalog/forms.json")).unwrap();
            for name in forms[0]["fields"]
                .as_object()
                .unwrap()
                .keys()
                .filter(|n| *n != "due")
            {
                assert!(rows[0].2.iter().any(|(p, _)| p == name), "{name}");
            }

            // an unknown form and an unknown board are told, not guessed
            let e = page(&source.client, "nope@1.0", "").await.unwrap_err();
            assert!(e.to_string().contains("form-source-unknown-form"), "{e}");
            let e = page(&source.client, FORM_CARDS, "no-such")
                .await
                .unwrap_err();
            assert!(e.to_string().contains("form-source-unknown-scope"), "{e}");

            // moveCard through invoke, every value a text
            let mut invoke = source.client.invoke_request();
            {
                let mut p = invoke.get();
                p.set_command("move");
                p.set_record(uid.as_str());
                let mut args = p.init_args(3);
                for (i, (k, v)) in [("board", "work"), ("column", "done"), ("index", "0")]
                    .into_iter()
                    .enumerate()
                {
                    let mut a = args.reborrow().get(u32::try_from(i).unwrap());
                    a.set_path(k);
                    a.set_text(v);
                }
            }
            let done = invoke.send().promise.await.unwrap();
            assert!(!done.get().unwrap().get_failed());
            let rows = page(&source.client, FORM_CARDS, "work").await.unwrap();
            assert_eq!(field(&rows[0], "column"), "done");

            // a column that does not exist and a command that does not exist: failed, with the reason
            for (command, column, why) in [
                ("move", "nowhere", ""),
                ("fly", "done", "form-source-unknown-command"),
            ] {
                let mut invoke = source.client.invoke_request();
                {
                    let mut p = invoke.get();
                    p.set_command(command);
                    p.set_record(uid.as_str());
                    let mut args = p.init_args(2);
                    for (i, (k, v)) in [("board", "work"), ("column", column)]
                        .into_iter()
                        .enumerate()
                    {
                        let mut a = args.reborrow().get(u32::try_from(i).unwrap());
                        a.set_path(k);
                        a.set_text(v);
                    }
                }
                let answer = invoke.send().promise.await.unwrap();
                let answer = answer.get().unwrap();
                assert!(answer.get_failed(), "{command}");
                let detail = answer
                    .get_problem()
                    .unwrap()
                    .get_type()
                    .unwrap()
                    .to_string()
                    .unwrap();
                assert!(detail.contains(why), "{detail}");
            }
        })
        .await;
}
