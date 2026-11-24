//! `watch` and `changes` with the scope of a board must also tell the DELETE of a card: the card is no longer in the layout after
//! the commit, so the scope of a gone card comes from the image it had before.
#![cfg(unix)]
#![allow(
    clippy::similar_names,
    clippy::many_single_char_names,
    clippy::unused_async_trait_impl
)]

mod common;

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use common::{board_with, call, serve};
use fs_interfaces::common_capnp::cursor;
use fs_interfaces::formsource_capnp::{change_ref, change_sink, form_source};
use tokio::task::LocalSet;

struct Heard(Rc<RefCell<Vec<(String, String)>>>);

impl change_sink::Server for Heard {
    async fn changed(
        self: Rc<Self>,
        params: change_sink::ChangedParams,
    ) -> Result<(), capnp::Error> {
        let c: change_ref::Reader<'_> = params.get()?.get_change()?;
        self.0
            .borrow_mut()
            .push((c.get_kind()?.to_string()?, c.get_object()?.to_string()?));
        Ok(())
    }
    async fn ended(
        self: Rc<Self>,
        _: change_sink::EndedParams,
        _: change_sink::EndedResults,
    ) -> Result<(), capnp::Error> {
        Ok(())
    }
}

#[tokio::test(flavor = "current_thread")]
async fn a_deleted_card_is_heard_by_a_watch_and_listed_by_changes_of_its_board() {
    let dir = tempfile::Builder::new().prefix("fs").tempdir().unwrap();
    LocalSet::new()
        .run_until(async {
            let (socket, _stop) = serve(dir.path()).await;
            let a = fs_rpc::connect::<form_source::Client>(&socket)
                .await
                .unwrap();
            let b = fs_rpc::connect::<form_source::Client>(&socket)
                .await
                .unwrap();
            let uids = board_with(&socket, &a.client, &["x", "y"]).await;
            let heard = Rc::new(RefCell::new(Vec::new()));
            let mut w = b.client.watch_request();
            {
                let mut p = w.get();
                p.set_form("cards-in-columns@1.2.0");
                p.set_scope("work");
                p.set_sink(capnp_rpc::new_client(Heard(heard.clone())));
            }
            let _sub = w.send().promise.await.unwrap();
            call(&a.client, "task.delete", &uids[0], &[]).await.unwrap();
            for _ in 0..50 {
                if heard.borrow().iter().any(|(k, _)| k == "Delete") {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
            let seen = heard.borrow().clone();
            assert!(
                seen.iter().any(|(k, o)| k == "Delete" && *o == uids[0]),
                "no Delete of the card was heard: {seen:?}"
            );
            // the same for `changes` of the board (the own program: none was introduced, so the empty program)
            let mut r = a.client.changes_request();
            {
                let mut p = r.get();
                p.set_scope("work");
                let _: cursor::Builder<'_> = p.init_cursor();
            }
            let items = r.send().promise.await.unwrap();
            let items = items.get().unwrap().get_items().unwrap();
            let kinds: Vec<_> = items
                .iter()
                .map(|c| {
                    (
                        c.get_kind().unwrap().to_string().unwrap(),
                        c.get_object().unwrap().to_string().unwrap(),
                    )
                })
                .collect();
            assert!(
                kinds.iter().any(|(k, o)| k == "Delete" && *o == uids[0]),
                "changes lost the Delete: {kinds:?}"
            );
        })
        .await;
}
