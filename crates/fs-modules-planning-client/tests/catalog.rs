//! The command catalog file of the module: commands by capability key of the contract, with their arguments, and the duties of the host
//! towards the file.

use fs_modules_planning_client::Catalog;

#[test]
fn the_shipped_catalog_names_the_commands_by_contract_key_with_their_arguments() {
    let catalog = Catalog::bundled();
    let keys: Vec<&str> = catalog.keys().collect();
    assert_eq!(keys, ["card.move", "task.create", "task.delete", "task.edit"]);
    let args = |key: &str| {
        let c = catalog.command(key).unwrap();
        c.params
            .iter()
            .map(|p| (p.path.as_str(), p.kind.as_str(), p.required))
            .collect::<Vec<_>>()
    };
    assert_eq!(
        catalog.command("card.move").unwrap().call,
        "Boards.moveCard"
    );
    assert_eq!(
        args("card.move"),
        [
            ("board", "text", true),
            ("column", "text", true),
            ("after", "reference", false),
            ("before", "reference", false)
        ]
    );
    assert_eq!(catalog.command("task.create").unwrap().call, "Items.create");
    assert_eq!(
        args("task.create"),
        [
            ("title", "text", true),
            ("board", "text", false),
            ("column", "text", true),
            ("after", "reference", false),
            ("before", "reference", false)
        ]
    );
    assert_eq!(catalog.command("task.edit").unwrap().call, "Items.edit");
    assert_eq!(
        args("task.edit"),
        [
            ("title", "text", false),
            ("description", "text", false),
            ("due", "text", false),
            ("priority", "text", false)
        ]
    );
    assert_eq!(catalog.command("task.delete").unwrap().call, "Items.delete");
    assert_eq!(args("task.delete"), Vec::new());
    // the old names, the prefixed ones and what no service serves are not in it
    for key in [
        "planning.move",
        "planning.card.move",
        "planning.create",
        "column.create",
        "task.claim",
        "claim",
    ] {
        assert!(catalog.command(key).is_none(), "{key}");
    }
}

#[test]
fn a_broken_catalog_is_refused_with_the_reason() {
    let command = |extra: &str| {
        format!(r#"{{"commands":[{{"capability":"a.b","call":"X.y","params":[]{extra}}}]}}"#)
    };
    assert!(Catalog::parse(&command("")).is_ok());
    assert!(
        Catalog::parse(&command(r#","id":"planning.a""#))
            .unwrap_err()
            .contains("`id`")
    );
    let twice = r#"{"commands":[{"capability":"a","call":"X.y","params":[]},{"capability":"a","call":"X.z","params":[]}]}"#;
    assert!(Catalog::parse(twice).unwrap_err().contains("twice"));
    let no_call = r#"{"commands":[{"capability":"a","params":[]}]}"#;
    assert!(Catalog::parse(no_call).unwrap_err().contains("call"));
    let no_params = r#"{"commands":[{"capability":"a","call":"X.y"}]}"#;
    assert!(Catalog::parse(no_params).unwrap_err().contains("params"));
    let param_twice = r#"{"commands":[{"capability":"a","call":"X.y","params":[{"path":"p","type":"text"},{"path":"p","type":"text"}]}]}"#;
    assert!(Catalog::parse(param_twice).unwrap_err().contains("twice"));
    let no_type = r#"{"commands":[{"capability":"a","call":"X.y","params":[{"path":"p"}]}]}"#;
    assert!(Catalog::parse(no_type).unwrap_err().contains("type"));
    assert!(Catalog::parse("{}").is_err());
    assert!(Catalog::parse("not json").is_err());
}

/// The duties the host puts on the file (fs-starter, from v0.0.7 on: `capability` = key of the contract, the id derived as
/// `<module>.<key>`, `params`; before: `registry/read.rs` + `registry/names.rs`); a stand-in until the one conformance test of every
/// module exists (card "fs-conformance" in the report).
#[test]
fn the_file_keeps_the_duties_of_the_host() {
    let text = include_str!("../../../catalog/commands.json");
    let doc: serde_json::Value = serde_json::from_str(text).unwrap();
    let module = fs_modules_planning_client::SOCKET_PROGRAM;
    assert_eq!(doc["module"], module);
    assert_eq!(doc["contract"], doc["module"]);
    assert_eq!(doc["scope"], "own-contract");
    // three parts (`semver`, version rule V); the params changed with the move by neighbours (after, before instead of index), so 0.0.3
    let version = semver::Version::parse(doc["version"].as_str().unwrap()).unwrap();
    assert_eq!(version, semver::Version::new(0, 0, 3));
    let word = |s: &str| {
        !s.is_empty()
            && s.bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-' || b == b'_')
    };
    let commands = doc["commands"].as_array().unwrap();
    assert_eq!(commands.len(), 4);
    for c in commands {
        let key = c["capability"].as_str().unwrap();
        assert!(c.get("id").is_none(), "{key}: no id, the host derives it");
        assert!(!key.starts_with(&format!("{module}.")), "{key}: no prefix");
        // the key is in the one list of the contract, and its parts are words (the derived id `planning.<key>` has two or more)
        assert!(fs_interfaces::capabilities::is_known(key), "{key}");
        assert!(key.split('.').all(word), "{key}");
        assert!(
            ["read", "write", "delete"].contains(&c["access"].as_str().unwrap()),
            "{key}"
        );
        for key in [
            &c["help"]["topic"],
            &c["help"]["text"]["en"],
            &c["help"]["text"]["de"],
        ] {
            assert!(!key.as_str().unwrap_or("").is_empty(), "{key}");
        }
    }
}

/// The service reports the keys of the contract, and every command of the file is one of them (the same word on both sides).
#[test]
fn the_file_and_the_service_use_the_same_keys() {
    let catalog = Catalog::bundled();
    for key in catalog.keys() {
        assert!(
            fs_modules_planning::CAPABILITIES.contains(&key),
            "{key}: the service does not report it"
        );
    }
    for id in fs_modules_planning::CAPABILITIES {
        assert!(
            !id.starts_with("planning."),
            "{id}: the service reports contract ids"
        );
    }
}

/// ADR 0104 E9: the planning module asks the module `person` for the owner of its store, and says so in its catalog.
#[test]
fn the_shipped_catalog_requires_the_module_person() {
    let requires = Catalog::bundled().requires().to_vec();
    assert_eq!(requires.len(), 1, "{requires:?}");
    assert_eq!(
        (requires[0].module.as_str(), requires[0].required),
        ("person", true)
    );
}

#[test]
fn a_broken_requires_is_refused_with_its_reason() {
    for (text, why) in [
        (r#"{"commands":[],"requires":"person"}"#, "no list"),
        (r#"{"commands":[],"requires":[{}]}"#, "without `module`"),
        (
            r#"{"commands":[],"requires":[{"module":"person","required":"yes"}]}"#,
            "no boolean",
        ),
    ] {
        let err = Catalog::parse(text).unwrap_err();
        assert!(err.contains(why), "{err}");
    }
    assert_eq!(Catalog::parse(r#"{"commands":[]}"#).unwrap().requires(), []);
}
