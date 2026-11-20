//! The contract itself: who acts, compare-and-set, an object that is no board, an unknown board.

mod common;

use fs_interfaces::tasks_capnp::{boards, items, tasks};
use tokio::task::LocalSet;

async fn root(service: &common::Service) -> (fs_rpc::Connection<tasks::Client>, boards::Client) {
    let connection = fs_rpc::connect::<tasks::Client>(&service.socket).await.unwrap();
    let boards = connection.client.boards_request().send().pipeline.get_boards();
    (connection, boards)
}

fn id_of(b: fs_interfaces::tasks_capnp::board::Reader<'_>) -> (u64, u64) {
    let id = b.get_envelope().unwrap().get_id().unwrap();
    (id.get_hi(), id.get_lo())
}

fn set_id(mut out: fs_interfaces::common_capnp::uuid::Builder<'_>, id: (u64, u64)) {
    out.set_hi(id.0);
    out.set_lo(id.1);
}

#[tokio::test(flavor = "current_thread")]
async fn the_actor_is_the_user_of_the_connection_not_the_store_and_not_a_field() {
    LocalSet::new()
        .run_until(async {
            let service = common::start().await;
            let (connection, _) = root(&service).await;
            let items: items::Client = connection.client.items_request().send().pipeline.get_items();
            let mut request = items.create_request();
            // A client that claims to be somebody else in the content: the envelope does not follow it.
            request.get().set_jscalendar(
                r#"{"@type":"Task","uid":"urn:uuid:01938f3e-0000-7000-8000-000000000001","title":"t","updated":"2026-11-20T10:00:00Z","attributedTo":"https://evil.invalid/root","by":"https://evil.invalid/root"}"#,
            );
            let response = request.send().promise.await.unwrap();
            let who = response
                .get()
                .unwrap()
                .get_task()
                .unwrap()
                .get_envelope()
                .unwrap()
                .get_attributed_to()
                .unwrap()
                .to_string()
                .unwrap();
            assert!(who.starts_with("urn:freesynergy:os-user:"), "{who}");
            assert!(who.len() > "urn:freesynergy:os-user:".len(), "an id from the system: {who}");
            assert_ne!(who, common::STORE_ACTOR, "the store's own actor must not be used for a connection");
            assert!(!who.contains("evil"), "{who}");
        })
        .await;
}

#[tokio::test(flavor = "current_thread")]
async fn a_stale_revision_is_a_conflict_and_writes_nothing() {
    LocalSet::new()
        .run_until(async {
            let service = common::start().await;
            let (_connection, boards) = root(&service).await;
            let mut create = boards.create_request();
            create.get().set_name("B");
            let made = create.send().promise.await.unwrap();
            let board = made.get().unwrap().get_board().unwrap();
            let (id, revision) = (id_of(board), board.get_revision());
            let edit = |revision: u64, name: &'static str| {
                let mut request = boards.edit_columns_request();
                let mut p = request.get();
                set_id(p.reborrow().init_id(), id);
                p.set_revision(revision);
                let mut columns = p.init_columns(1);
                let mut c = columns.reborrow().get(0);
                c.set_id("only");
                c.set_name(name);
                request.send().promise
            };
            let first = edit(revision, "First").await.unwrap();
            let new_revision = first.get().unwrap().get_board().unwrap().get_revision();
            assert_ne!(new_revision, revision, "a write changes the revision");
            let e = edit(revision, "Stale").await.err().expect("a stale revision must fail");
            assert!(e.to_string().contains("conflict:"), "{e}");
            let list = boards.list_request().send().promise.await.unwrap();
            let board = list.get().unwrap().get_boards().unwrap().get(0);
            assert_eq!(board.get_columns().unwrap().get(0).get_name().unwrap().to_str().unwrap(), "First");
        })
        .await;
}

#[tokio::test(flavor = "current_thread")]
async fn a_board_that_is_not_valid_is_rejected() {
    LocalSet::new()
        .run_until(async {
            let service = common::start().await;
            let (_connection, boards) = root(&service).await;
            let mut create = boards.create_request();
            create.get().set_name("B");
            let made = create.send().promise.await.unwrap();
            let board = made.get().unwrap().get_board().unwrap();
            let mut request = boards.edit_columns_request();
            let mut p = request.get();
            set_id(p.reborrow().init_id(), id_of(board));
            p.set_revision(board.get_revision());
            let _ = p.init_columns(0); // no column at all
            let e = request.send().promise.await.err().expect("a board without a column is refused");
            assert!(e.to_string().contains("rejected:"), "{e}");
        })
        .await;
}

#[tokio::test(flavor = "current_thread")]
async fn the_id_of_a_task_or_of_nothing_is_no_board() {
    LocalSet::new()
        .run_until(async {
            let service = common::start().await;
            let (connection, boards) = root(&service).await;
            // A line the board does not own: a task object (another type in the same store).
            let items: items::Client = connection.client.items_request().send().pipeline.get_items();
            let mut request = items.create_request();
            request.get().set_jscalendar(
                r#"{"@type":"Task","uid":"urn:uuid:01938f3e-0000-7000-8000-000000000002","title":"t","updated":"2026-11-20T10:00:00Z"}"#,
            );
            let task = request.send().promise.await.unwrap();
            let t = task.get().unwrap().get_task().unwrap().get_envelope().unwrap().get_id().unwrap();
            let task_id = (t.get_hi(), t.get_lo());
            for id in [task_id, (0x0193_8f3e_0000_7000, 0x8000_0000_0000_00ff)] {
                let mut layout = boards.layout_request();
                set_id(layout.get().init_id(), id);
                assert!(layout.send().promise.await.is_err(), "layout of {id:?}");
                let mut moving = boards.move_card_request();
                let mut p = moving.get();
                set_id(p.reborrow().init_id(), id);
                p.set_uid("urn:uuid:x");
                p.set_column("todo");
                assert!(moving.send().promise.await.is_err(), "moveCard on {id:?}");
            }
        })
        .await;
}
