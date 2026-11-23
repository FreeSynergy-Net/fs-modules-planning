//! The command catalog file of the module: command id to contract call, and the duties of the host towards the file.

use fs_modules_planning_client::Catalog;

#[test]
fn the_shipped_catalog_maps_the_commands_to_the_calls_of_the_decision() {
    let catalog = Catalog::bundled();
    let call = |id: &str| {
        catalog
            .command(id)
            .map(|c| (c.call.as_str(), c.capability.as_str()))
    };
    assert_eq!(
        call("planning.move"),
        Some(("Boards.moveCard", "planning.card.move"))
    );
    // only what a service serves: `FormSource.invoke` serves `planning.move`; create, column.create, delete come with the write path (card SCHREIBWEG)
    assert_eq!(catalog.ids().count(), 1);
    for id in [
        "planning.create",
        "planning.column.create",
        "planning.delete",
        "planning.claim",
        "planning.release",
        "planning.note",
        "claim",
    ] {
        assert!(catalog.command(id).is_none(), "{id}");
    }
}

#[test]
fn a_broken_catalog_is_refused_with_the_reason() {
    let twice = r#"{"commands":[{"id":"a","call":"X.y","capability":"c"},{"id":"a","call":"X.z","capability":"c"}]}"#;
    assert!(Catalog::parse(twice).unwrap_err().contains("twice"));
    let no_call = r#"{"commands":[{"id":"a","capability":"c"}]}"#;
    assert!(Catalog::parse(no_call).unwrap_err().contains("call"));
    assert!(Catalog::parse("{}").is_err());
    assert!(Catalog::parse("not json").is_err());
}

/// The duties the host (fs-starter v0.0.5, `registry/read.rs` + `registry/names.rs`) puts on the file; a stand-in until the one
/// conformance test of every module exists (card "fs-conformance" in the report).
#[test]
fn the_file_keeps_the_duties_of_the_host() {
    let text = include_str!("../../../catalog/commands.json");
    let doc: serde_json::Value = serde_json::from_str(text).unwrap();
    let module = fs_modules_planning_client::SOCKET_PROGRAM;
    assert_eq!(doc["module"], module);
    assert_eq!(doc["contract"], doc["module"]);
    assert_eq!(doc["scope"], "own-contract");
    assert_eq!(doc["version"], "0.0.1");
    let word = |s: &str| {
        !s.is_empty()
            && s.bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-' || b == b'_')
    };
    let commands = doc["commands"].as_array().unwrap();
    assert_eq!(commands.len(), 1, "planning.move only");
    let space = format!("{module}.");
    for c in commands {
        let id = c["id"].as_str().unwrap();
        assert!(id.starts_with(&space), "{id}");
        assert!(
            id.split('.').count() >= 2 && id.split('.').all(word),
            "{id}"
        );
        assert!(
            c["capability"].as_str().unwrap().starts_with(&space),
            "{id}"
        );
        assert!(
            ["read", "write", "delete"].contains(&c["access"].as_str().unwrap()),
            "{id}"
        );
        for key in [
            &c["help"]["topic"],
            &c["help"]["text"]["en"],
            &c["help"]["text"]["de"],
        ] {
            assert!(!key.as_str().unwrap_or("").is_empty(), "{id}");
        }
    }
}

/// The file names the capability with the prefix of the module (the host asks for it there); the service reports the ids of the
/// contract. The two worlds meet in `contract_capability`, and every command of the file is one the service reports.
#[test]
fn the_file_and_the_service_use_two_kinds_of_ids_and_they_meet() {
    let catalog = Catalog::bundled();
    for id in catalog.ids() {
        let c = catalog.command(id).unwrap();
        assert!(c.capability.starts_with("planning."), "{id}");
        let contract = c.contract_capability();
        assert!(!contract.starts_with("planning."), "{id}: {contract}");
        assert!(
            fs_modules_planning::CAPABILITIES.contains(&contract),
            "{id}: the service does not report {contract}"
        );
    }
    for id in fs_modules_planning::CAPABILITIES {
        assert!(
            !id.starts_with("planning."),
            "{id}: the service reports contract ids"
        );
    }
}
