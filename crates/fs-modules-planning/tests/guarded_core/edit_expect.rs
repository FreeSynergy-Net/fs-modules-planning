//! `task.edit` takes no `expect` yet: it is refused with 501 and nothing is written (never silently dropped); without `expect` it
//! writes with the revision it read.
//!
//! Pattern: Scenario test (given a card, when an edit comes with `expect`, then it is refused and the task stays).

use super::fixture::world;
use super::{cx, inv};
use fs_interfaces::module::ModuleCore;
use fs_modules_planning::PlanningCore;
use fs_tasks_core::TaskItems;
use tokio::task::LocalSet;

#[tokio::test(flavor = "current_thread")]
async fn an_expect_on_task_edit_is_refused_with_501_and_writes_nothing() {
    LocalSet::new()
        .run_until(async {
            let w = world().await;
            let core = PlanningCore::new(w.repo.clone());
            let uid = &w.uids[0];
            let before = w.repo.get_by_uid(uid).await.unwrap().unwrap().task.title;
            let mut edit = inv("task.edit", uid, &[("title", "Changed")]);
            edit.expect = Some("1".into());
            let refused = core.apply(&cx(), edit).await.unwrap_err();
            assert_eq!(refused.status, 501, "{refused:?}");
            let after = w.repo.get_by_uid(uid).await.unwrap().unwrap().task.title;
            assert_eq!(before, after, "nothing was written");
            let ok = core
                .apply(&cx(), inv("task.edit", uid, &[("title", "Changed")]))
                .await;
            assert!(ok.is_ok(), "{ok:?}");
        })
        .await;
}
