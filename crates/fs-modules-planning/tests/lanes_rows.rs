//! `rows@1.0.0` of the planning module: the lanes of a swim-lane view (type `Lane`, scope `lanes:<axis>[:<board>]`, also the empty
//! ones stored at the board).
//!
//! Pattern: Projection (the module lists the lanes and the values of the standard; the frontend groups and names them).
#![cfg(unix)]

mod common;

use common::{field, page, serve_repo};
use fs_interfaces::formsource_capnp::form_source;
use fs_interfaces::tasks_capnp::tasks;
use fs_tasks_core::model::ActorType;
use fs_tasks_core::{Boards, Task};
use tokio::task::LocalSet;

const ROWS: &str = "rows@1.0.0";
const ADA: &str = "freesynergy.net:person:ada";
const BOB: &str = "freesynergy.net:person:bob";
const ZED: &str = "freesynergy.net:person:zed";

async fn ids(source: &form_source::Client, scope: &str) -> Vec<String> {
    let rows = page(source, ROWS, scope).await.unwrap();
    rows.iter().map(|r| r.0.clone()).collect()
}

#[tokio::test(flavor = "current_thread")]
async fn lanes_come_from_the_axis_and_the_stored_ones_come_first() {
    let dir = tempfile::Builder::new().prefix("fs").tempdir().unwrap();
    LocalSet::new()
        .run_until(async {
            let (socket, _stop, repo) = serve_repo(dir.path()).await;
            let source = fs_rpc::connect::<form_source::Client>(&socket)
                .await
                .unwrap();
            let tasks = fs_rpc::connect::<tasks::Client>(&socket).await.unwrap();
            let boards = tasks.client.boards_request().send().pipeline.get_boards();
            for name in ["work", "home"] {
                let mut create = boards.create_request();
                create.get().set_name(name);
                create.send().promise.await.unwrap();
            }
            let items = tasks.client.items_request().send().pipeline.get_items();
            let mut one = Task::new("One");
            one.assign_actor(ADA, ActorType::Person, "Ada");
            let mut both = Task::new("Both");
            both.assign_actor(ADA, ActorType::Person, "Ada");
            both.assign_actor(BOB, ActorType::Person, "Bob");
            let mut failed = Task::new("Failed");
            failed.progress = Some("failed".into());
            for task in [one, both, failed, Task::new("Nobody")] {
                let mut add = items.create_request();
                add.get()
                    .set_jscalendar(task.to_value().to_string().as_str());
                add.send().promise.await.unwrap();
            }
            // an empty lane of the board: what the contract cannot write, so it is stored through the repository
            let entry = repo
                .boards()
                .await
                .unwrap()
                .into_iter()
                .find(|b| b.board.name == "work")
                .unwrap();
            let mut board = entry.board.clone();
            board.lanes.insert("assignee".into(), vec![ZED.into()]);
            repo.board_edit(entry.id, entry.revision(), board)
                .await
                .unwrap();

            // status: the five words of the RFC in order
            let five = [
                "needs-action",
                "in-process",
                "completed",
                "failed",
                "cancelled",
            ];
            assert_eq!(ids(&source.client, "lanes:state").await, five);
            // people: the stored lane first (empty), then everyone in charge of a task of the board, each once
            assert_eq!(
                ids(&source.client, "lanes:assignee:work").await,
                [ZED, ADA, BOB]
            );
            // no board named = every board: the stored lane of `work` is still there
            assert_eq!(ids(&source.client, "lanes:assignee").await, [ZED, ADA, BOB]);
            // boards: all, or the one named
            assert_eq!(ids(&source.client, "lanes:lane").await.len(), 2);
            assert_eq!(ids(&source.client, "lanes:lane:home").await, ["home"]);
            // the record: name, place and axis
            let rows = page(&source.client, ROWS, "lanes:assignee:work")
                .await
                .unwrap();
            assert_eq!(field(&rows[0], "label"), ZED);
            assert_eq!(field(&rows[2], "order"), "2");
            assert_eq!(field(&rows[2], "axis"), "assignee");
            // an axis that does not exist, and a form that is not served
            assert!(page(&source.client, ROWS, "lanes:colour").await.is_err());
            assert!(
                page(&source.client, ROWS, "lanes:assignee:nowhere")
                    .await
                    .is_err()
            );
            assert!(
                page(&source.client, "rows@2.0.0", "lanes:state")
                    .await
                    .is_err()
            );
        })
        .await;
}
