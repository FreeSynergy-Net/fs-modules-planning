//! `task.create` without `board` (the form's `lane` is optional on create): exactly one board is meant, none or several is refused
//! with the reason (`form-source-bad-arguments`, 400), never guessed.
#![cfg(unix)]

mod common;

use common::{call, field, page, serve};
use fs_interfaces::formsource_capnp::form_source;
use fs_interfaces::tasks_capnp::tasks;
use fs_modules_planning::FORM_CARDS;
use tokio::task::LocalSet;

async fn add_board(socket: &std::path::Path, name: &str) {
    let tasks = fs_rpc::connect::<tasks::Client>(socket).await.unwrap();
    let boards = tasks.client.boards_request().send().pipeline.get_boards();
    let mut create = boards.create_request();
    create.get().set_name(name);
    create.send().promise.await.unwrap();
}

const NO_BOARD: [(&str, &str); 2] = [("title", "Probe"), ("column", "todo")];

#[tokio::test(flavor = "current_thread")]
async fn create_without_board_takes_the_only_board_and_refuses_none_or_several_with_a_reason() {
    let dir = tempfile::Builder::new().prefix("fs").tempdir().unwrap();
    LocalSet::new()
        .run_until(async {
            let (socket, _stop) = serve(dir.path()).await;
            let source = fs_rpc::connect::<form_source::Client>(&socket)
                .await
                .unwrap();

            // no board at all
            let (kind, why) = call(&source.client, "task.create", "", &NO_BOARD)
                .await
                .unwrap_err();
            assert_eq!(kind, "form-source-bad-arguments", "{why}");
            assert!(why.contains("no board"), "{why}");

            // exactly one board: the card goes there, an empty `board` counts as missing
            add_board(&socket, "work").await;
            let with_empty = [("title", "Probe"), ("column", "todo"), ("board", "")];
            call(&source.client, "task.create", "", &with_empty)
                .await
                .unwrap();
            let rows = page(&source.client, FORM_CARDS, "work").await.unwrap();
            assert_eq!(rows.len(), 1);
            assert_eq!(field(&rows[0], "title"), "Probe");

            // two boards: refused, the boards are named, nothing is created
            add_board(&socket, "home").await;
            let (kind, why) = call(&source.client, "task.create", "", &NO_BOARD)
                .await
                .unwrap_err();
            assert_eq!(kind, "form-source-bad-arguments", "{why}");
            assert!(why.contains("work") && why.contains("home"), "{why}");
            assert_eq!(
                page(&source.client, FORM_CARDS, "work")
                    .await
                    .unwrap()
                    .len(),
                1
            );
        })
        .await;
}
