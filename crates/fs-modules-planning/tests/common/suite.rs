//! The fixture of the suite `module_v1` for planning: the declared forms and commands of the catalog files and the valid calls, for a
//! client of the service (over a socket in `conformance.rs`, over `Guarded<PlanningCore>` in `guarded_core`). ONE fixture for both.
//!
//! Pattern: Template Method (the rules are the suite's; this file is the fixture).

use fs_interfaces::formsource_capnp::form_source;
use fs_module::{
    CommandKind, DeclaredCommand, DeclaredForm, FormSourceFixture, Ordered, Param, Sample,
    ValueField, ValueKind, Verify,
};

pub const FORM: &str = "cards-in-columns@1.4.0";

/// The suite's view of planning: two connections of the same service.
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
