//! The seed of the examples against a real service: `page(cards-in-columns@1.4.0)` after the seed has every value that was set and none
//! that was left out.
//!
//! Pattern: Object Mother (the sample data of `examples/seed.rs`) with an end-to-end read.

mod common;

#[allow(dead_code)]
#[path = "../examples/seed.rs"]
mod seed;

use fs_interfaces::common_capnp::cursor;
use fs_interfaces::formsource_capnp::form_source;
use fs_interfaces::tasks_capnp::tasks;
use fs_modules_planning_client::Planning;
use tokio::task::LocalSet;

type Values = Vec<(String, String)>;

async fn cards(source: &form_source::Client) -> Vec<Values> {
    let mut request = source.page_request();
    {
        let mut p = request.get();
        p.set_form("cards-in-columns@1.4.0");
        p.set_limit(100);
        let _: cursor::Builder<'_> = p.init_cursor();
    }
    let answer = request.send().promise.await.unwrap();
    let records = answer
        .get()
        .unwrap()
        .get_page()
        .unwrap()
        .get_records()
        .unwrap();
    records
        .iter()
        .map(|r| {
            r.get_values()
                .unwrap()
                .iter()
                .map(|v| {
                    (
                        v.get_path().unwrap().to_string().unwrap(),
                        v.get_text().unwrap().to_string().unwrap(),
                    )
                })
                .collect()
        })
        .collect()
}

fn all<'a>(card: &'a Values, path: &str) -> Vec<&'a str> {
    card.iter()
        .filter(|(p, _)| p == path)
        .map(|(_, v)| v.as_str())
        .collect()
}

#[tokio::test(flavor = "current_thread")]
async fn the_seed_sets_the_values_and_leaves_the_empty_ones_out() {
    LocalSet::new()
        .run_until(async {
            let service = common::start().await;
            let conn = fs_rpc::connect::<tasks::Client>(&service.socket)
                .await
                .unwrap();
            let mut planning = Planning::over(conn.client.clone()).await.unwrap();
            seed::seed(&conn.client, &mut planning).await;
            let source = fs_rpc::connect::<form_source::Client>(&service.socket)
                .await
                .unwrap();
            let page = cards(&source.client).await;
            assert_eq!(page.len(), seed::cards().len());
            for s in seed::cards() {
                let card = page
                    .iter()
                    .find(|c| all(c, "title") == [s.title])
                    .unwrap_or_else(|| panic!("{}", s.title));
                let one = |v: Option<String>| v.into_iter().collect::<Vec<_>>();
                assert_eq!(
                    all(card, "priority"),
                    one(s.priority.map(|p| p.to_string())),
                    "{} priority",
                    s.title
                );
                assert_eq!(
                    all(card, "estimatedDuration"),
                    one(s.estimated.map(Into::into)),
                    "{} estimated",
                    s.title
                );
                assert_eq!(
                    all(card, "timeZone"),
                    one(s.zone.map(Into::into)),
                    "{} zone",
                    s.title
                );
                assert_eq!(all(card, "assignee"), s.who, "{} assignee", s.title);
                assert_eq!(all(card, "progress"), [s.progress], "{} state", s.title);
            }
            // the card with both people repeats the value; the failed task has the word `failed`
            assert_eq!(
                page.iter()
                    .filter(|c| all(c, "assignee").len() == 2)
                    .count(),
                1
            );
            assert_eq!(
                page.iter()
                    .filter(|c| all(c, "assignee").is_empty())
                    .count(),
                5
            );
            assert_eq!(
                page.iter()
                    .filter(|c| all(c, "progress") == ["failed"])
                    .count(),
                1
            );
            // one floating card: a due without a zone
            assert_eq!(
                page.iter()
                    .filter(|c| !all(c, "due").is_empty() && all(c, "timeZone").is_empty())
                    .count(),
                1
            );
        })
        .await;
}
