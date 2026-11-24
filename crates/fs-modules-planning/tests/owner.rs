//! The owner of the service: read once from the source before anything is bound or opened (`serve_with`, Strategy `LocalOwner`).
//! Without a "me" the service does not start and says `owner-no-me` on stderr; there is no anonymous owner.
#![cfg(unix)]

use fs_interfaces::owner::OwnerProblem;
use fs_interfaces::owner::fake::NoOwner;
use fs_modules_planning::serve_with;

fn args(dir: &std::path::Path) -> Vec<std::ffi::OsString> {
    vec![
        "serve".into(),
        "--socket".into(),
        dir.join("planning.sock").into(),
        "--data".into(),
        dir.join("data").into(),
    ]
}

#[test]
fn no_owner_means_the_service_does_not_start_and_says_owner_no_me() {
    let dir = tempfile::Builder::new().prefix("fs").tempdir().unwrap();
    let owner = NoOwner(OwnerProblem::NoMe {
        hint: "create the person \"me\" in the module person".into(),
    });
    let mut stderr = Vec::new();
    let code = serve_with(args(dir.path()), &owner, &mut stderr);
    let text = String::from_utf8(stderr).unwrap();
    assert_eq!(code, 1, "{text}");
    assert!(text.contains("owner-no-me"), "{text}");
    // nothing was bound, nothing was opened
    assert!(!dir.path().join("planning.sock").exists());
    assert!(!dir.path().join("data").exists());
}

#[test]
fn a_missing_source_of_the_owner_is_told_with_its_own_code() {
    let dir = tempfile::Builder::new().prefix("fs").tempdir().unwrap();
    let owner = NoOwner(OwnerProblem::Unreachable {
        reason: "no answer".into(),
    });
    let mut stderr = Vec::new();
    assert_eq!(serve_with(args(dir.path()), &owner, &mut stderr), 1);
    assert!(
        String::from_utf8(stderr)
            .unwrap()
            .contains("owner-unreachable")
    );
}

#[test]
fn help_needs_no_owner() {
    let owner = NoOwner(OwnerProblem::NoMe { hint: "x".into() });
    let mut stderr = Vec::new();
    assert_eq!(serve_with(["--help".into()], &owner, &mut stderr), 0);
    assert_eq!(stderr.len(), 0);
}
