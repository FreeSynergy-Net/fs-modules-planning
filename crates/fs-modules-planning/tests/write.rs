//! The write commands over a real service in a temporary folder: `task.create` (between neighbours of a FULL column), `card.move` (neighbours, uid
//! checks, never a number) and `task.delete`, each with its success, its refusals and what the answer names.
#![cfg(unix)]

mod common;

use common::{board_with, call, invoke, serve, titles};
use fs_interfaces::formsource_capnp::form_source;
use fs_interfaces::tasks_capnp::tasks;
use tokio::task::LocalSet;

/// The arguments of a `task.create` into `todo` of `work`, with the neighbours `near`.
fn make<'a>(title: &'a str, near: &[(&'a str, &'a str)]) -> Vec<(&'a str, &'a str)> {
    let mut args = vec![("title", title), ("board", "work"), ("column", "todo")];
    args.extend_from_slice(near);
    args
}

/// The arguments of a `card.move` into `column` of `work`, with the neighbours `near`.
fn to<'a>(column: &'a str, near: &[(&'a str, &'a str)]) -> Vec<(&'a str, &'a str)> {
    let mut args = vec![("board", "work"), ("column", column)];
    args.extend_from_slice(near);
    args
}

async fn count_tasks(socket: &std::path::Path) -> usize {
    let tasks = fs_rpc::connect::<tasks::Client>(socket).await.unwrap();
    let items = tasks.client.items_request().send().pipeline.get_items();
    let list = items.list_request().send().promise.await.unwrap();
    list.get()
        .unwrap()
        .get_page()
        .unwrap()
        .get_items()
        .unwrap()
        .len() as usize
}

#[tokio::test(flavor = "current_thread")]
async fn a_task_is_created_between_neighbours_of_a_full_column_and_deleted() {
    let dir = tempfile::Builder::new().prefix("fs").tempdir().unwrap();
    LocalSet::new()
        .run_until(async {
            let (socket, _stop) = serve(dir.path()).await;
            let source = fs_rpc::connect::<form_source::Client>(&socket)
                .await
                .unwrap();
            let uids = board_with(&socket, &source.client, &["a", "b", "c"]).await;
            let source = source.client;
            // the column is full (three cards): the neighbours decide the place, none = the end
            let first = invoke(
                &source,
                "task.create",
                "",
                &make("  front ", &[("before", &uids[0])]),
            )
            .await;
            assert!(!first.failed, "{first:?}");
            assert_eq!(first.text, "task-created");
            let middle = invoke(
                &source,
                "task.create",
                "",
                &make("middle", &[("after", &uids[0]), ("before", &uids[1])]),
            )
            .await;
            let end = invoke(&source, "task.create", "", &make("end", &[])).await;
            assert!(!middle.failed && !end.failed);
            assert_eq!(
                titles(&source).await,
                [
                    "todo/front",
                    "todo/a",
                    "todo/middle",
                    "todo/b",
                    "todo/c",
                    "todo/end"
                ]
            );
            // the answer names the new card: it is the record the page shows
            let rows = common::page(&source, "cards-in-columns@1.4.0", "work")
                .await
                .unwrap();
            assert_eq!(rows[0].0, first.record);
            assert_eq!(rows[2].0, middle.record);
            assert_eq!(rows[5].0, end.record);
            assert_eq!(count_tasks(&socket).await, 6);

            // refused: nothing is made (the count of tasks stays), each with its own kind
            let n = count_tasks(&socket).await;
            let refused = [
                (
                    make("x", &[])
                        .into_iter()
                        .map(|(k, v)| {
                            if k == "column" {
                                (k, "nowhere")
                            } else {
                                (k, v)
                            }
                        })
                        .collect::<Vec<_>>(),
                    "form-source-rejected",
                ),
                (
                    make("x", &[])
                        .into_iter()
                        .map(|(k, v)| if k == "board" { (k, "none") } else { (k, v) })
                        .collect::<Vec<_>>(),
                    "form-source-unknown-scope",
                ),
                (make("  ", &[]), "form-source-bad-arguments"),
                (make("x", &[("index", "0")]), "form-source-bad-arguments"),
                (make("x", &[("order", "1")]), "form-source-bad-arguments"),
            ];
            for (args, why) in &refused {
                let (kind, detail) = call(&source, "task.create", "", args).await.unwrap_err();
                assert_eq!(&kind, why, "{args:?}: {detail}");
            }
            // a create takes no record: one is not silently dropped
            let (kind, _) = call(&source, "task.create", &uids[0], &make("x", &[]))
                .await
                .unwrap_err();
            assert_eq!(kind, "form-source-bad-arguments");
            assert_eq!(count_tasks(&socket).await, n);

            // delete: the card goes, the answer has no record; a second delete, an unknown and a too long uid are told apart
            let gone = invoke(&source, "task.delete", &uids[1], &[]).await;
            assert!(!gone.failed, "{gone:?}");
            assert_eq!(
                (gone.record.as_str(), gone.text.as_str()),
                ("", "task-deleted")
            );
            assert!(!titles(&source).await.contains(&"todo/b".to_owned()));
            for (uid, why) in [
                (uids[1].clone(), "form-source-unknown-record"),
                ("no-such-uid".to_owned(), "form-source-unknown-record"),
                ("u".repeat(300), "form-source-bad-arguments"),
                (String::new(), "form-source-bad-arguments"),
            ] {
                let (kind, _) = call(&source, "task.delete", &uid, &[]).await.unwrap_err();
                assert_eq!(kind, why, "{uid}");
            }
            assert_eq!(count_tasks(&socket).await, n - 1);
        })
        .await;
}

#[tokio::test(flavor = "current_thread")]
async fn a_card_moves_between_neighbours_in_a_full_column_and_a_wrong_uid_is_told() {
    let dir = tempfile::Builder::new().prefix("fs").tempdir().unwrap();
    LocalSet::new()
        .run_until(async {
            let (socket, _stop) = serve(dir.path()).await;
            let source = fs_rpc::connect::<form_source::Client>(&socket)
                .await
                .unwrap();
            let uids = board_with(&socket, &source.client, &["a", "b", "c"]).await;
            let source = source.client;
            // within the full column: c between a and b, then a to the end (right after b)
            call(
                &source,
                "card.move",
                &uids[2],
                &to("todo", &[("after", &uids[0]), ("before", &uids[1])]),
            )
            .await
            .unwrap();
            assert_eq!(titles(&source).await, ["todo/a", "todo/c", "todo/b"]);
            call(
                &source,
                "card.move",
                &uids[0],
                &to("todo", &[("after", &uids[1])]),
            )
            .await
            .unwrap();
            assert_eq!(titles(&source).await, ["todo/c", "todo/b", "todo/a"]);
            // into another column, then behind the card that stands there
            call(&source, "card.move", &uids[0], &to("doing", &[]))
                .await
                .unwrap();
            call(
                &source,
                "card.move",
                &uids[1],
                &to("doing", &[("after", &uids[0])]),
            )
            .await
            .unwrap();
            assert_eq!(titles(&source).await, ["todo/c", "doing/a", "doing/b"]);
            // the uid is checked: unknown, too long, empty are three different refusals
            let args = to("done", &[]);
            for (uid, why) in [
                ("no-such-uid".to_owned(), "form-source-unknown-record"),
                ("u".repeat(300), "form-source-bad-arguments"),
                (String::new(), "form-source-bad-arguments"),
            ] {
                let (kind, _) = call(&source, "card.move", &uid, &args).await.unwrap_err();
                assert_eq!(kind, why, "{uid}");
            }
            // a place by a number is never taken (S1 E3), whatever the number; a neighbour that is gone does not refuse
            for number in [("index", "-1"), ("index", "0"), ("order", "1")] {
                let args = [("board", "work"), ("column", "done"), number];
                let (kind, why) = call(&source, "card.move", &uids[0], &args)
                    .await
                    .unwrap_err();
                assert_eq!(kind, "form-source-bad-arguments", "{number:?}");
                assert!(why.contains("never by a number"), "{why}");
            }
            call(
                &source,
                "card.move",
                &uids[0],
                &to("done", &[("after", "gone")]),
            )
            .await
            .unwrap();
            // the card itself as its own neighbour is no neighbour: it goes to the end of the column
            call(
                &source,
                "card.move",
                &uids[0],
                &to("doing", &[("after", &uids[0])]),
            )
            .await
            .unwrap();
            assert_eq!(titles(&source).await, ["todo/c", "doing/b", "doing/a"]);
        })
        .await;
}
