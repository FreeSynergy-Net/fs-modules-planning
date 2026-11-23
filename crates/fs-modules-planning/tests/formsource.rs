//! `FormSource` next to `Tasks` on one socket, over a real service in a temporary folder: hello, a page of cards by column, and the
//! first write: `move` through `invoke` with nothing but text values.
#![cfg(unix)]

mod common;

use common::{field, page, serve};
use fs_interfaces::formsource_capnp::form_source;
use fs_interfaces::tasks_capnp::tasks;
use fs_modules_planning::{FORM_CARDS, PROVIDER};
use fs_tasks_core::Task;
use tokio::task::LocalSet;

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
            for name in forms[0]["fields"].as_object().unwrap().keys().filter(|n| {
                !["due", "priority", "participants", "keywords", "description"]
                    .contains(&n.as_str())
            }) {
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
                p.set_command("planning.move");
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
                ("planning.move", "nowhere", ""),
                ("planning.fly", "done", "form-source-unknown-command"),
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
