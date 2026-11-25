//! A connection for which the service knows no person gets nothing from `FormSource` (403, fail closed), and nothing is written.
//! Over a real socket and the real `Planning` (the person of a connection is the port of `Guarded`, see `root.rs`).
//!
//! Pattern: Scenario test (given a service without a person for the caller, when it writes and reads, then 403 and no change).
#![cfg(unix)]

mod common;

use common::{call, page, serve_for};
use fs_interfaces::formsource_capnp::form_source;
use tokio::task::LocalSet;

const FORM: &str = "cards-in-columns@1.4.0";

#[tokio::test(flavor = "current_thread")]
async fn a_connection_without_a_known_person_is_refused_and_writes_nothing() {
    let dir = tempfile::Builder::new().prefix("fs").tempdir().unwrap();
    LocalSet::new()
        .run_until(async {
            let (socket, _stop, repo) = serve_for(dir.path(), false).await;
            let source = fs_rpc::connect::<form_source::Client>(&socket)
                .await
                .unwrap();
            let args = [("title", "x"), ("board", "work"), ("column", "todo")];
            let (kind, _) = call(&source.client, "task.create", "", &args)
                .await
                .unwrap_err();
            assert_eq!(kind, "form-source-forbidden");
            let read = page(&source.client, FORM, "work").await;
            assert!(
                read.is_err() || read.unwrap().is_empty(),
                "a page was given"
            );
            let made = fs_tasks_core::TaskItems::list(&*repo).await.unwrap();
            assert!(made.is_empty(), "something was written");
        })
        .await;
}
