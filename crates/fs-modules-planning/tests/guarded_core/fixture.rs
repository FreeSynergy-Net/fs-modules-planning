//! The store for the suite `module_v1` over `Guarded<PlanningCore>`: a temporary folder with the board `work`, three cards and the
//! seeded channels (the fixture itself is `common::suite::Planning`).
//!
//! Pattern: Object Mother (a ready store for the tests).
#![allow(dead_code)]

use std::sync::Arc;

use fs_interfaces::owner::LocalOwner as _;
use fs_interfaces::owner::fake::FixedOwner;
use fs_objects::{Store, TursoStore};
use fs_tasks_core::model::ActorType;
use fs_tasks_core::{Boards, Task, TaskItems, TaskRepo};

pub use super::common::suite::FORM;

/// A store with the board `work` (standard columns), the cards `a`, `b`, `c` in `todo` and the seeded channels.
pub struct World {
    pub repo: Arc<TaskRepo>,
    pub uids: Vec<String>,
    dir: tempfile::TempDir,
}

/// An empty store (no board, no card) whose change channel holds `capacity` changes (a small one makes a slow feed lag).
pub async fn empty(capacity: usize) -> World {
    let dir = tempfile::Builder::new().prefix("fs").tempdir().unwrap();
    let owner = FixedOwner(fs_objects::testing::owner()).owner().unwrap();
    let mut options = fs_objects::OpenOptions::default().with_owner(owner.clone());
    options.subscribe_capacity = capacity;
    let store = TursoStore::open_with(dir.path().join("objects.db"), options)
        .await
        .unwrap();
    let store: Arc<dyn Store> = Arc::new(store);
    let repo = TaskRepo::new(store, "https://local.invalid/users/owner", owner);
    World {
        repo: Arc::new(repo),
        uids: Vec::new(),
        dir,
    }
}

pub async fn world() -> World {
    let World { repo, dir, .. } = empty(1024).await;
    let repo = Arc::into_inner(repo).expect("one handle");
    let board = repo.board_create("work", None).await.unwrap();
    let mut uids = Vec::new();
    for title in ["a", "b", "c"] {
        uids.push(repo.create(Task::new(title)).await.unwrap().task.uid);
    }
    // a limit of 3 on `todo`, and one task with every own channel
    let mut edited = board.board.clone();
    for column in &mut edited.columns {
        column.wip_limit = (column.id == "todo").then_some(3);
    }
    repo.board_edit(board.id, board.revision(), edited)
        .await
        .unwrap();
    let mut full = Task::new("full");
    full.priority = Some(1);
    full.description = Some("why".into());
    full.estimated_duration = Some("PT1H30M".into());
    full.set_vendor("color", serde_json::json!("#ff8800"));
    full.time_zone = Some("Europe/Berlin".into());
    full.assign_actor("freesynergy.net:person:ada", ActorType::Person, "Ada");
    full.assign_actor("freesynergy.net:person:bob", ActorType::Person, "Bob");
    repo.create(full).await.unwrap();
    World {
        repo: Arc::new(repo),
        uids,
        dir,
    }
}
