//! The fixture of the suite `module_v1` for planning over `Guarded<PlanningCore>`: a store in a temporary folder with the board
//! `work`, three cards and the seeded channels, and the declared forms and commands of the catalog files.
//!
//! Pattern: Template Method (the rules are the suite's; this file is the fixture, as in `tests/conformance.rs`).
#![allow(dead_code)]

use std::sync::Arc;

use fs_interfaces::formsource_capnp::form_source;
use fs_interfaces::owner::LocalOwner as _;
use fs_interfaces::owner::fake::FixedOwner;
use fs_module::{
    CommandKind, DeclaredCommand, DeclaredForm, FormSourceFixture, Ordered, Param, Sample,
    ValueField, ValueKind, Verify,
};
use fs_objects::{Store, TursoStore};
use fs_tasks_core::model::ActorType;
use fs_tasks_core::{Boards, Task, TaskItems, TaskRepo};

pub const FORM: &str = "cards-in-columns@1.4.0";

/// A store with the board `work` (standard columns), the cards `a`, `b`, `c` in `todo` and the seeded channels.
pub struct World {
    pub repo: Arc<TaskRepo>,
    pub uids: Vec<String>,
    _dir: tempfile::TempDir,
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
        _dir: dir,
    }
}

pub async fn world() -> World {
    let World { repo, _dir, .. } = empty(1024).await;
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
        _dir,
    }
}

/// The suite's view of planning: two connections of the same wrapper.
pub struct Planning {
    pub client: form_source::Client,
    pub other: form_source::Client,
    pub uids: Vec<String>,
}

impl FormSourceFixture for Planning {
    fn client(&self) -> form_source::Client {
        self.client.clone()
    }

    fn level(&self) -> u32 {
        fs_modules_planning::LEVEL
    }

    fn second(&self) -> Option<form_source::Client> {
        Some(self.other.clone())
    }

    fn provider(&self) -> Option<&str> {
        Some("planning")
    }

    fn forms(&self) -> Vec<DeclaredForm> {
        let forms: serde_json::Value =
            serde_json::from_str(include_str!("../../../../catalog/forms.json")).unwrap();
        vec![DeclaredForm {
            key: FORM.into(),
            scope: "work".into(),
            fields: forms[0]["fields"]
                .as_object()
                .unwrap()
                .keys()
                .cloned()
                .collect(),
        }]
    }

    fn commands(&self) -> Vec<DeclaredCommand> {
        let file: serde_json::Value =
            serde_json::from_str(include_str!("../../../../catalog/commands.json")).unwrap();
        file["commands"]
            .as_array()
            .unwrap()
            .iter()
            .map(|c| {
                let key = c["capability"].as_str().unwrap().to_owned();
                DeclaredCommand {
                    kind: if key == "task.create" {
                        CommandKind::Set
                    } else {
                        CommandKind::Record
                    },
                    params: c["params"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(|p| Param {
                            name: p["path"].as_str().unwrap().to_owned(),
                            required: p["required"] == true,
                        })
                        .collect(),
                    key,
                }
            })
            .collect()
    }

    fn samples(&self) -> Vec<Sample> {
        let a = |k: &str, v: &str| (k.to_owned(), v.to_owned());
        let value = |path: &str, text: &str| Verify::Value {
            form: FORM.into(),
            record: None,
            path: path.into(),
            text: text.into(),
        };
        vec![
            Sample {
                command: "card.move".into(),
                record: self.uids[2].clone(),
                args: vec![
                    a("board", "work"),
                    a("column", "doing"),
                    a("after", ""),
                    a("before", ""),
                ],
                makes_record: true,
                verify: value("column", "doing"),
            },
            Sample {
                command: "task.create".into(),
                record: String::new(),
                args: vec![
                    a("title", "Made"),
                    a("board", "work"),
                    a("column", "todo"),
                    a("before", &self.uids[0]),
                ],
                makes_record: true,
                verify: value("title", "Made"),
            },
            Sample {
                command: "task.delete".into(),
                record: self.uids[1].clone(),
                args: vec![],
                makes_record: false,
                verify: Verify::Gone {
                    form: FORM.into(),
                    record: self.uids[1].clone(),
                },
            },
        ]
    }

    fn strict_scope(&self) -> bool {
        true
    }

    fn ordered(&self) -> Option<Ordered> {
        Some(Ordered {
            command: "card.move".into(),
            form: FORM.into(),
            record: self.uids[2].clone(),
            neighbour: self.uids[0].clone(),
            args: vec![
                ("board".into(), "work".into()),
                ("column".into(), "todo".into()),
            ],
        })
    }

    fn value_fields(&self) -> Vec<ValueField> {
        [
            ("description", ValueKind::Description),
            ("color", ValueKind::Color),
            ("estimatedDuration", ValueKind::Estimated),
            ("progress", ValueKind::State),
            ("timeZone", ValueKind::TimeZone),
            ("assignee", ValueKind::Assignee),
            ("column-limit", ValueKind::ColumnLimit),
            ("priority", ValueKind::Priority),
        ]
        .into_iter()
        .map(|(path, kind)| ValueField {
            form: FORM.into(),
            path: path.into(),
            kind,
        })
        .collect()
    }
}
