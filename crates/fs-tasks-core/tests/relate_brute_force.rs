#![allow(clippy::pedantic)]
//! `relate` against a brute force: it refuses exactly the relations that close a dependency cycle among the stored tasks,
//! writes nothing when it refuses, and names a real way round. A cycle through a task the store does not hold is not
//! seen by `relate` (it never was; see DESIGN.md).

mod common;

use std::collections::BTreeSet;

use common::random::{RELS, Rng, random_task, store};
use common::repo;
use fs_tasks_core::plan::{Edge, edges_of, path_closing_cycle};
use fs_tasks_core::{Error, Relation, Task, TaskItems};

fn closes_cycle(all: &[Task], holder: &str, other: &str, rel: Relation) -> (bool, bool) {
    // (stored-only paths as v0.0.5 saw them, any path incl. ghost nodes)
    let Some(e) = Edge::of_relation(holder, other, rel, chrono::Duration::zero()) else {
        return (false, false);
    };
    let edges = edges_of(all);
    let any = path_closing_cycle(&edges, &e).is_some();
    let stored: BTreeSet<&str> = all.iter().map(|t| t.uid.as_str()).collect();
    let inner: Vec<Edge> = edges
        .iter()
        .filter(|x| stored.contains(x.from.as_str()) && stored.contains(x.to.as_str()))
        .cloned()
        .collect();
    let st = e.from == e.to
        || (stored.contains(e.from.as_str())
            && stored.contains(e.to.as_str())
            && path_closing_cycle(&inner, &e).is_some());
    (st, any)
}

async fn relate_runs(seed: u64, runs: usize) -> (Vec<String>, usize, usize) {
    let mut rng = Rng(seed);
    let mut findings = Vec::new();
    let (mut wides, mut refusals) = (0, 0);
    for run in 0..runs {
        let wide = rng.below(8) == 0;
        wides += usize::from(wide);
        let n = if wide {
            30 + rng.below(30)
        } else {
            2 + rng.below(7)
        };
        let ghosts = rng.below(2);
        let max_rel = if wide { 4 } else { 2 };
        let tasks: Vec<Task> = (0..n)
            .map(|i| random_task(&mut rng, i, n, ghosts, max_rel))
            .collect();
        let r = repo().await;
        let stored = store(&r, &tasks).await;
        let all: Vec<Task> = stored.values().map(|e| e.task.clone()).collect();
        for _ in 0..3 {
            let holder = format!("u{}", rng.below(n));
            let other = format!("u{}", rng.below(n + ghosts));
            let rel = RELS[rng.below(5)];
            let he = &stored[&holder];
            let before = r.get(he.id).await.unwrap().unwrap();
            let (st, any) = closes_cycle(&all, &holder, &other, rel);
            let res = if rel.is_temporal() && rng.below(2) == 0 {
                r.relate_with_gap(he.id, &other, rel, "P1D").await
            } else {
                r.relate(he.id, &other, rel).await
            };
            let refused = matches!(res, Err(Error::Cycle(_)));
            if let Err(e) = &res
                && !refused
                && findings.len() < 10
            {
                findings.push(format!("[ERR] seed {seed} run {run}: {e:?}"));
            }
            if refused {
                refusals += 1;
                let after = r.get(he.id).await.unwrap().unwrap();
                if serde_json::to_value(&after.task).unwrap()
                    != serde_json::to_value(&before.task).unwrap()
                {
                    findings.push(format!(
                        "[ATOMIC] seed {seed} run {run}: written although refused"
                    ));
                }
                if let Err(Error::Cycle(path)) = &res {
                    // path must be a real way from the successor to the predecessor
                    let e =
                        Edge::of_relation(&holder, &other, rel, chrono::Duration::zero()).unwrap();
                    let edges = edges_of(&all);
                    let ok = path.first() == Some(&e.to)
                        && path.last() == Some(&e.from)
                        && path
                            .windows(2)
                            .all(|w| edges.iter().any(|x| x.from == w[0] && x.to == w[1]));
                    if !ok && findings.len() < 10 {
                        findings.push(format!(
                            "[PATH] seed {seed} run {run}: {path:?} for {holder} {rel:?} {other}"
                        ));
                    }
                }
                continue;
            }
            if st != refused || (any && !refused) {
                let tag = if st == refused {
                    "GHOST-only (also in v0.0.5)"
                } else {
                    "REAL"
                };
                if findings.len() < 10 || tag == "REAL" {
                    findings.push(format!(
                        "[RELATE {tag}] seed {seed} run {run} wide {wide}: {holder} {rel:?} {other}: stored-cycle {st} any-cycle {any} refused {refused}"
                    ));
                }
            }
            // stop after the first accepted change, the stored picture is stale now
            if res.is_ok() {
                break;
            }
        }
    }
    (findings, wides, refusals)
}

/// Why 80 and not 400 in the standard run: in a debug build under CI load the 400 runs took up to 120 s (the nextest limit).
/// 80 runs with the fixed seeds below still meet every kind of graph and both answers (the test asserts that wide graphs and refusals are met); the full 400 run in the nextest
/// profile `release` (see `common::random::runs`) and by hand with `REV_RUNS=400 cargo test`.
/// Pattern: Bounded Exhaustive Check – a small fixed-seed sample of the random space in the standard run, the full range in `release`.
const QUICK_RUNS: usize = 80;

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn relate_refuses_exactly_the_cycles() {
    let runs = common::random::runs(QUICK_RUNS);
    let parts = 8;
    let mut handles = Vec::new();
    for p in 0..parts {
        handles.push(tokio::spawn(relate_runs(
            0xD1B5_4A32_D192_ED03 ^ ((p as u64 + 3) * 104_729),
            runs / parts,
        )));
    }
    let mut findings = Vec::new();
    let (mut wides, mut refusals) = (0, 0);
    for h in handles {
        let (f, w, r) = h.await.unwrap();
        findings.extend(f);
        wides += w;
        refusals += r;
    }
    assert!(
        wides > 0 && refusals > 0,
        "wide graphs ({wides}) and refusals ({refusals}) are met"
    );
    let real = findings.iter().filter(|f| !f.contains("GHOST")).count();
    eprintln!(
        "RELATE runs {runs}: findings {} (real {real})",
        findings.len()
    );
    for f in findings.iter().take(20) {
        eprintln!("{f}");
    }
    assert_eq!(real, 0);
}
