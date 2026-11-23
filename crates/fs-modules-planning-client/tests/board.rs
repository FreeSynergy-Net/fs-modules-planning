//! The host's view of the board over a real service: columns, cards, moves, new columns, limits, capabilities.

mod common;

use fs_modules_planning_client::{Kind, Operation, Planning};
use tokio::task::LocalSet;

fn ids(columns: &[fs_modules_planning_client::Column]) -> Vec<&str> {
    columns.iter().map(|c| c.id.as_str()).collect()
}

#[tokio::test(flavor = "current_thread")]
async fn a_fresh_store_gets_a_board_with_the_standard_columns() {
    LocalSet::new()
        .run_until(async {
            let service = common::start().await;
            let mut planning = Planning::connect(&service.socket).await.unwrap();
            let columns = planning.columns().await.unwrap();
            assert_eq!(ids(&columns), ["todo", "doing", "done", "closed"]);
            assert!(columns.iter().all(|c| c.cards.is_empty() && !c.over_limit));
        })
        .await;
}

#[tokio::test(flavor = "current_thread")]
async fn the_service_reports_what_the_host_may_unlock() {
    LocalSet::new()
        .run_until(async {
            let service = common::start().await;
            let planning = Planning::connect(&service.socket).await.unwrap();
            for op in [
                Operation::AddColumn,
                Operation::Rename,
                Operation::Limit,
                Operation::Move,
                Operation::AddCard,
            ] {
                assert!(planning.can(op), "{op:?}");
            }
            assert!(planning.can_command("planning.move"));
            for id in [
                "planning.create",
                "planning.column.create",
                "planning.delete",
            ] {
                assert!(!planning.can_command(id), "{id}: no service behind it");
            }
            assert!(
                !planning.can_command("planning.claim"),
                "no such command: the catalog does not list it, the service does not serve it"
            );
        })
        .await;
}

#[tokio::test(flavor = "current_thread")]
async fn a_card_is_made_in_its_column_and_moved() {
    LocalSet::new()
        .run_until(async {
            let service = common::start().await;
            let mut planning = Planning::connect(&service.socket).await.unwrap();
            let card = planning.add_card("doing", "Write the test").await.unwrap();
            planning.add_card("doing", "Second").await.unwrap();
            let columns = planning.columns().await.unwrap();
            let doing = columns.iter().find(|c| c.id == "doing").unwrap();
            let titles: Vec<_> = doing.cards.iter().map(|c| c.title.as_str()).collect();
            assert_eq!(titles, ["Write the test", "Second"]);
            planning
                .move_card(&card.uid, "done", Some(0))
                .await
                .unwrap();
            let columns = planning.columns().await.unwrap();
            let in_column = |id: &str| -> Vec<String> {
                columns
                    .iter()
                    .find(|c| c.id == id)
                    .unwrap()
                    .cards
                    .iter()
                    .map(|c| c.uid.clone())
                    .collect()
            };
            assert_eq!(in_column("done"), std::slice::from_ref(&card.uid));
            assert_eq!(in_column("doing").len(), 1);
        })
        .await;
}

#[tokio::test(flavor = "current_thread")]
async fn a_card_in_an_unknown_column_is_refused() {
    LocalSet::new()
        .run_until(async {
            let service = common::start().await;
            let mut planning = Planning::connect(&service.socket).await.unwrap();
            let e = planning.add_card("nowhere", "x").await.unwrap_err();
            assert_eq!(e.kind, Kind::Rejected, "{e}");
        })
        .await;
}

#[tokio::test(flavor = "current_thread")]
async fn columns_are_added_renamed_and_limited() {
    LocalSet::new()
        .run_until(async {
            let service = common::start().await;
            let mut planning = Planning::connect(&service.socket).await.unwrap();
            assert_eq!(planning.add_column("Review").await.unwrap(), "review");
            assert_eq!(planning.add_column("Review").await.unwrap(), "review-2");
            planning
                .rename_column("review-2", "Second review")
                .await
                .unwrap();
            planning.set_limit("todo", Some(1)).await.unwrap();
            planning.add_card("todo", "one").await.unwrap();
            planning.add_card("todo", "two").await.unwrap();
            let columns = planning.columns().await.unwrap();
            assert_eq!(ids(&columns)[4..], ["review", "review-2"]);
            assert_eq!(columns[5].name, "Second review");
            assert_eq!(columns[0].limit, Some(1));
            assert!(columns[0].over_limit, "two cards, a limit of one: reported");
            assert_eq!(columns[0].cards.len(), 2, "never refused");
            planning.set_limit("todo", None).await.unwrap();
            assert!(!planning.columns().await.unwrap()[0].over_limit);
        })
        .await;
}

#[tokio::test(flavor = "current_thread")]
async fn an_empty_column_name_and_an_unknown_column_are_refused() {
    LocalSet::new()
        .run_until(async {
            let service = common::start().await;
            let mut planning = Planning::connect(&service.socket).await.unwrap();
            let e = planning.add_column("  ").await.unwrap_err();
            assert_eq!(e.kind, Kind::Rejected);
            let e = planning.rename_column("nope", "x").await.unwrap_err();
            assert_eq!(e.kind, Kind::Rejected);
            // Nothing was written.
            assert_eq!(planning.columns().await.unwrap().len(), 4);
        })
        .await;
}

#[tokio::test(flavor = "current_thread")]
async fn nothing_listens_is_unreachable() {
    let dir = tempfile::tempdir().unwrap();
    let missing = dir.path().join("none.sock");
    LocalSet::new()
        .run_until(async {
            let e = Planning::connect(&missing).await.err().unwrap();
            assert_eq!(e.kind, Kind::Unreachable);
        })
        .await;
}
