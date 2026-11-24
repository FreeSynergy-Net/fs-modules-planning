//! The shipped path of the owner: the program asks the person service of the module person (the REAL binary, started by the
//! program itself through `ensure_single`), and without a running "me" it does not start. Folders are temporary; every child and the
//! person service are ended at the end of the test, also when it fails.
#![cfg(unix)]

#[path = "../common/mod.rs"]
mod common;
mod home;

use std::time::{Duration, Instant};

use fs_interfaces::formsource_capnp::form_source;
use fs_interfaces::tasks_capnp::tasks;
use fs_modules_planning::serve_with;
use fs_person_service::{COMMAND_CREATE, PersonServiceOwner};
use fs_rpc::{RestartTimes, SingleTimes};

use home::{Home, finish, local};

/// P-owner 1: the module person runs, but "me" is not created: the shipped program does not start and says `owner-no-me`.
#[test]
fn the_program_does_not_start_without_me_in_the_running_module_person() {
    let home = Home::new();
    let out = finish(home.command().spawn().unwrap(), Duration::from_secs(30));
    let text = String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status.code(), Some(1), "{text}");
    assert!(text.contains("owner-no-me"), "{text}");
    // nothing was bound, nothing was opened
    assert!(!home.planning_socket().exists());
    assert!(
        !home
            .0
            .path()
            .join("data/freesynergy/modules/planning")
            .exists()
    );
}

/// P-owner 2: with "me" the same program starts and serves, and a second start on the same socket is served by the first.
#[test]
fn the_program_starts_and_serves_when_me_exists() {
    let home = Home::new();
    let first = finish(home.command().spawn().unwrap(), Duration::from_secs(30));
    assert_eq!(first.status.code(), Some(1), "no me yet");
    let me = local(async {
        let c = fs_rpc::connect::<form_source::Client>(&home.person_socket())
            .await
            .unwrap();
        let made = common::invoke(&c.client, COMMAND_CREATE, "", &[]).await;
        assert!(!made.failed, "{}", made.detail);
        made.record
    });
    assert!(me.starts_with("urn:uuid:"), "{me}");
    let mut child = home.command().spawn().unwrap();
    local(async {
        let until = Instant::now() + Duration::from_secs(30);
        while fs_rpc::connect::<tasks::Client>(&home.planning_socket())
            .await
            .is_err()
        {
            assert!(Instant::now() < until, "the program did not come up");
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        fs_rpc::ask_beat(&home.planning_socket(), Duration::from_secs(2))
            .await
            .expect("the heartbeat answers");
    });
    assert!(child.try_wait().unwrap().is_none(), "it keeps serving");
    let _ = child.kill();
    let _ = child.wait();
}

fn short() -> SingleTimes {
    let ms = Duration::from_millis;
    SingleTimes {
        handshake: RestartTimes {
            call: ms(300),
            stop: ms(500),
            start: ms(800),
        },
        overall: Duration::from_secs(4),
        ..SingleTimes::default()
    }
}

/// P-owner 3: a module person that cannot be reached (no program to start) is `owner-unreachable` with the socket as the reason,
/// and nothing is bound or opened.
#[test]
fn an_unreachable_module_person_stops_the_service_with_its_reason() {
    let dir = tempfile::Builder::new().prefix("po").tempdir().unwrap();
    let person_socket = dir.path().join("p.sock");
    let owner =
        PersonServiceOwner::new(&person_socket, dir.path().join("none")).with_times(short());
    let args = [
        "serve".into(),
        "--socket".into(),
        dir.path().join("planning.sock").into(),
        "--data".into(),
        dir.path().join("data").into(),
    ];
    let mut stderr = Vec::new();
    assert_eq!(serve_with(args, &owner, &mut stderr), 1);
    let text = String::from_utf8(stderr).unwrap();
    assert!(text.contains("owner-unreachable"), "{text}");
    assert!(
        text.contains("p.sock"),
        "the reason names the socket: {text}"
    );
    assert!(!dir.path().join("planning.sock").exists());
    assert!(!dir.path().join("data").exists());
}
