//! Seed and dump the planning service over its contract: the cards of a picture, with the values the form `cards-in-columns@1.4.0`
//! sends (priority, estimated duration, time zone, people in charge, a failed task). Run: `seed <socket>` (fills the board of a fresh
//! store) or `dump <socket>` (prints the board).
//!
//! Pattern: Object Mother (fixed sample data, the same on every run). The contract has no call for the lanes stored at a board
//! (`Board::lanes`, `editColumns` carries columns and places only), so the stored empty lane of a person is not seeded here.
use fs_interfaces::tasks_capnp::tasks;
use fs_modules_planning_client::Planning;
use fs_tasks_core::Task;
use fs_tasks_core::model::ActorType;

/// The two people in charge (`chair`).
pub const ADA: &str = "freesynergy.net:person:ada";
pub const BOB: &str = "freesynergy.net:person:bob";

/// One card of the seed: its title, the index of its column, `due`, status word and percent, and the optional values.
pub struct Seed {
    pub title: &'static str,
    pub column: usize,
    pub due: &'static str,
    pub progress: &'static str,
    pub percent: u8,
    pub priority: Option<i32>,
    pub estimated: Option<&'static str>,
    /// `Some(zone)`, or `None` = floating (no `timeZone`).
    pub zone: Option<&'static str>,
    pub who: &'static [&'static str],
}

const fn card(
    title: &'static str,
    column: usize,
    due: &'static str,
    progress: &'static str,
    percent: u8,
) -> Seed {
    Seed {
        title,
        column,
        due,
        progress,
        percent,
        priority: None,
        estimated: None,
        zone: Some("UTC"),
        who: &[],
    }
}

/// The cards: priority 1, 2, 3, 5, 9 and one without; the durations PT30M, PT1H, PT2H30M, PT4H and two without; the zone Europe/Berlin on
/// two cards, one floating, the rest UTC; one card with both people, one with Ada, the others with nobody; one failed task.
#[must_use]
pub fn cards() -> Vec<Seed> {
    vec![
        Seed {
            priority: Some(1),
            estimated: Some("PT30M"),
            zone: Some("Europe/Berlin"),
            who: &[ADA, BOB],
            ..card(
                "Write the picture report",
                0,
                "2026-11-26T12:00:00",
                "needs-action",
                0,
            )
        },
        Seed {
            priority: Some(2),
            estimated: Some("PT1H"),
            zone: Some("Europe/Berlin"),
            who: &[ADA],
            ..card(
                "Order the wall paint",
                0,
                "2026-11-30T09:00:00",
                "needs-action",
                0,
            )
        },
        Seed {
            priority: Some(3),
            estimated: Some("PT2H30M"),
            zone: None,
            ..card(
                "Book the print shop",
                0,
                "2026-12-02T09:00:00",
                "needs-action",
                0,
            )
        },
        Seed {
            priority: Some(5),
            estimated: Some("PT4H"),
            ..card(
                "Review the sample pages",
                1,
                "2026-11-25T17:00:00",
                "in-process",
                40,
            )
        },
        Seed {
            priority: Some(9),
            ..card(
                "Fix the print settings",
                1,
                "2026-11-27T10:00:00",
                "in-process",
                75,
            )
        },
        card(
            "Choose the new logo",
            2,
            "2026-11-20T08:00:00",
            "completed",
            100,
        ),
        card("Rent the stage", 3, "2026-11-22T08:00:00", "failed", 10),
    ]
}

/// Create the cards of [`cards`] on the board of the service and put each in its column.
pub async fn seed(conn: &tasks::Client, p: &mut Planning) {
    let ids: Vec<String> = p
        .columns()
        .await
        .expect("columns")
        .iter()
        .map(|c| c.id.clone())
        .collect();
    for s in cards() {
        let mut task = Task::new(s.title);
        task.due = Some(s.due.into());
        task.time_zone = s.zone.map(Into::into);
        task.progress = Some(s.progress.into());
        task.percent_complete = Some(s.percent);
        task.priority = s.priority;
        task.estimated_duration = s.estimated.map(Into::into);
        for who in s.who {
            task.assign_actor(
                who,
                ActorType::Person,
                who.rsplit(':').next().unwrap_or(who),
            );
        }
        let items = conn.items_request().send().pipeline.get_items();
        let mut r = items.create_request();
        r.get().set_jscalendar(task.to_value().to_string().as_str());
        r.send().promise.await.expect("create");
        p.move_card(&task.uid, &ids[s.column], None)
            .await
            .expect("move");
    }
}

#[allow(clippy::missing_panics_doc)]
#[tokio::main(flavor = "current_thread")]
async fn main() {
    let mut a = std::env::args().skip(1);
    let mode = a.next().expect("mode seed|dump");
    let socket = std::path::PathBuf::from(a.next().expect("socket"));
    tokio::task::LocalSet::new()
        .run_until(async move {
            let conn = fs_rpc::connect::<tasks::Client>(&socket)
                .await
                .expect("connect");
            let mut p = Planning::over(conn.client.clone()).await.expect("over");
            if mode == "seed" {
                seed(&conn.client, &mut p).await;
            }
            for c in p.columns().await.expect("columns") {
                println!("{} ({}):", c.name, c.id);
                for k in &c.cards {
                    println!("   {} | {}", k.uid, k.title);
                }
            }
        })
        .await;
}
