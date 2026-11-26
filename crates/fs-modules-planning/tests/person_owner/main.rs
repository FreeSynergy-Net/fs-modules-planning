//! The shipped path of the owner: the program asks the person service of the module person (the REAL binary, started by the
//! program itself through `ensure_single`), and "me" is made by that service at its start. Folders are temporary; every child and the
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

use home::{Home, local};

/// P-owner 1/2 (since person 0.0.3): the person service births "me" itself at its first start, so the shipped program starts and
/// serves at once, with no `me.create` from anybody; `me.create` afterwards finds the same person (`me-exists`). (Without "me" the
/// program still ends with `owner-no-me`: that is shown with an owner that has none, not with the real person service, which
/// does not run without "me" any more.)
#[test]
fn the_program_starts_and_serves_with_the_me_the_person_service_made_at_its_start() {
    let home = Home::new();
    let mut child = home.command().spawn().unwrap();
    local(async {
        let until = Instant::now() + Duration::from_secs(30);
        // "Up" means: the socket accepts AND the heartbeat answers. The service opens its heartbeat as the first step after binding
        // (serve.rs `start_beat`), but a caller can still be quicker than that one step, so both are waited for with the same limit
        // (the old test asked the heartbeat once, the moment the socket accepted, and was red when it was not there yet).
        loop {
            let served = fs_rpc::connect::<tasks::Client>(&home.planning_socket())
                .await
                .is_ok();
            if served
                && fs_rpc::ask_beat(&home.planning_socket(), Duration::from_secs(2))
                    .await
                    .is_ok()
            {
                break;
            }
            assert!(
                Instant::now() < until,
                "the program did not come up (socket accepts: {served}, heartbeat answers: no)"
            );
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    });
    assert!(child.try_wait().unwrap().is_none(), "it keeps serving");
    let again = local(async {
        let c = fs_rpc::connect::<form_source::Client>(&home.person_socket())
            .await
            .unwrap();
        common::invoke(&c.client, COMMAND_CREATE, "", &[]).await
    });
    assert!(!again.failed, "{}", again.detail);
    assert_eq!(again.text, "me-exists");
    assert!(again.record.starts_with("urn:uuid:"), "{}", again.record);
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
