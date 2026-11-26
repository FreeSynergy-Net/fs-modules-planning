//! The form source level and the version of the required module in `catalog/commands.json`.
//!
//! Pattern: Contract test – the file is the declaration (it becomes `Requires: freesynergy-form-source >= N` and
//! `Requires: fs-module-person >= x.y.z`), `fs_interfaces::forms::FORM_SOURCE_LEVEL` is the contract. The packager cannot
//! compare the level with the constant, so this test does; it also keeps `at_least` in the shape the packager demands.

use serde_json::Value;

fn file() -> Value {
    serde_json::from_str(include_str!("../../../catalog/commands.json"))
        .expect("catalog/commands.json is not valid JSON")
}

#[test]
fn declared_form_source_level_is_the_level_of_the_contract() {
    let declared = file()["form_source_level"]
        .as_u64()
        .expect("`form_source_level` is not a whole number");
    assert!(declared >= 1, "the level must be >= 1");
    assert_eq!(declared, u64::from(fs_interfaces::forms::FORM_SOURCE_LEVEL));
}

#[test]
fn every_at_least_is_a_plain_x_y_z() {
    let file = file();
    let rows = file["requires"].as_array().expect("`requires` is no list");
    let person = rows
        .iter()
        .find(|r| r["module"] == "person")
        .expect("planning requires the module person");
    assert_eq!(person["at_least"], "0.0.3");
    for row in rows {
        let Some(v) = row.get("at_least") else {
            continue;
        };
        let v = v.as_str().expect("`at_least` is not text");
        let parts: Vec<&str> = v.split('.').collect();
        assert_eq!(parts.len(), 3, "`{v}` is not x.y.z");
        assert!(
            parts
                .iter()
                .all(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit())),
            "`{v}` is not x.y.z (digits only, no pre-release)"
        );
        semver::Version::parse(v).expect("`at_least` is not a version");
    }
}
