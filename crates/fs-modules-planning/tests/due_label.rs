//! What a card carries for the time and the column: `due` as an RFC 3339 `date-time` WITH an offset (the form art `time`), the
//! column name beside its id, and the percent. Over a real service in a temporary folder.
#![cfg(unix)]

mod common;

use common::{field, page, serve};
use fs_interfaces::formsource_capnp::form_source;
use fs_interfaces::tasks_capnp::tasks;
use fs_modules_planning::FORM_CARDS;
use fs_tasks_core::Task;
use tokio::task::LocalSet;

#[tokio::test(flavor = "current_thread")]
async fn due_is_an_instant_with_offset_and_the_column_shows_its_name() {
    let dir = tempfile::Builder::new().prefix("fs").tempdir().unwrap();
    LocalSet::new()
        .run_until(async {
            let (socket, _stop) = serve(dir.path()).await;
            let source = fs_rpc::connect::<form_source::Client>(&socket)
                .await
                .unwrap();
            let tasks = fs_rpc::connect::<tasks::Client>(&socket).await.unwrap();
            let boards = tasks.client.boards_request().send().pipeline.get_boards();
            let mut create = boards.create_request();
            create.get().set_name("work");
            create.send().promise.await.unwrap();
            let items = tasks.client.items_request().send().pipeline.get_items();
            let mut berlin = Task::new("Zoned");
            berlin.due = Some("2026-11-26T12:00:00".into());
            berlin.time_zone = Some("Europe/Berlin".into());
            berlin.percent_complete = Some(40);
            let mut floating = Task::new("Floating");
            floating.due = Some("2026-11-26T12:00:00".into());
            for task in [berlin, floating, Task::new("Bare")] {
                let mut add = items.create_request();
                add.get()
                    .set_jscalendar(task.to_value().to_string().as_str());
                add.send().promise.await.unwrap();
            }
            let rows = page(&source.client, FORM_CARDS, "work").await.unwrap();
            let row = |title: &str| rows.iter().find(|r| field(r, "title") == title).unwrap();
            // 12:00 in Berlin in November is 11:00 UTC; the floating one has no zone and is read in the floating zone (UTC)
            assert_eq!(field(row("Zoned"), "due"), "2026-11-26T11:00:00Z");
            assert_eq!(field(row("Floating"), "due"), "2026-11-26T12:00:00Z");
            assert_eq!(field(row("Bare"), "due"), "", "no due, no value");
            assert_eq!(field(row("Zoned"), "percentComplete"), "40");
            assert_eq!(field(row("Bare"), "percentComplete"), "");
            // the id stays the target of `move`, the name is what the head shows
            assert_eq!(field(row("Zoned"), "column"), "todo");
            assert_eq!(field(row("Zoned"), "column-label"), "To do");
        })
        .await;
}
