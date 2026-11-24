//! `FormSource` next to `Tasks` on one socket, over a real service in a temporary folder: hello, a page of cards by column, and the
//! first write: `card.move` through `invoke` with nothing but text values.
#![cfg(unix)]

mod common;

use common::{call, field, invoke, page, serve};
use fs_interfaces::formsource_capnp::form_source;
use fs_interfaces::tasks_capnp::tasks;
use fs_modules_planning::{FORM_CARDS, PROVIDER};
use fs_tasks_core::Task;
use tokio::task::LocalSet;

/// Fields a card has only when its task has them (or that the neighbours write, not the page).
const OPTIONAL: [&str; 11] = [
    "color",
    "estimated",
    "column-limit",
    "due",
    "percent",
    "priority",
    "participants",
    "keywords",
    "description",
    "after",
    "before",
];

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
            assert_eq!(hello.get_level(), fs_modules_planning::LEVEL);
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
            // both ways of writing the key, and the older minors (a subset of the form) are read, with the same cards
            for key in [
                "cards-in-columns@1.3",
                "cards-in-columns@1.2",
                "cards-in-columns@1.1.0",
                "cards-in-columns@1.1",
                "cards-in-columns@1.0.0",
                "cards-in-columns@1.0",
            ] {
                assert_eq!(
                    page(&source.client, key, "work").await.unwrap(),
                    rows,
                    "{key}"
                );
            }
            for key in [
                "cards-in-columns@1.4.0",
                "cards-in-columns@2.0.0",
                "cards-in-columns@x",
            ] {
                let e = page(&source.client, key, "work").await.unwrap_err();
                assert!(
                    e.to_string().contains("form-source-unknown-form"),
                    "{key}: {e}"
                );
            }
            let uid = rows[0].0.clone();

            // every field the catalog names is one the service writes (`due` only for a task that has one)
            let forms: serde_json::Value =
                serde_json::from_str(include_str!("../../../catalog/forms.json")).unwrap();
            for name in forms[0]["fields"]
                .as_object()
                .unwrap()
                .keys()
                .filter(|n| !OPTIONAL.contains(&n.as_str()))
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

            // card.move through invoke, every value a text; the answer names the card it moved
            let moved = invoke(
                &source.client,
                "card.move",
                &uid,
                &[("board", "work"), ("column", "done")],
            )
            .await;
            assert!(!moved.failed, "{moved:?}");
            assert_eq!(
                (moved.record.as_str(), moved.text.as_str()),
                (uid.as_str(), "card-moved")
            );
            let rows = page(&source.client, FORM_CARDS, "work").await.unwrap();
            assert_eq!(field(&rows[0], "column"), "done");

            // a column that does not exist is a refusal of the core (not a break of the module); a command with the prefix of the host
            // or one nobody serves is `unknown-command`: the wire name is the key of the contract
            let nowhere = [("board", "work"), ("column", "nowhere")];
            let (kind, why) = call(&source.client, "card.move", &uid, &nowhere)
                .await
                .unwrap_err();
            assert_eq!(kind, "form-source-rejected", "{why}");
            assert!(why.contains("nowhere"), "{why}");
            for command in ["planning.card.move", "planning.move", "card.fly"] {
                let (kind, _) = call(&source.client, command, &uid, &nowhere)
                    .await
                    .unwrap_err();
                assert_eq!(kind, "form-source-unknown-command", "{command}");
            }
        })
        .await;
}
