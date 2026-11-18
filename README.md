# FreeSynergy planning modules

FreeSynergy planning modules: dependencies (RFC 9253), critical path, hand order, checklists, progress states, claims and the to-do data module (JSCalendar Task, RFC 8984; VTODO, RFC 5545). Built with help from Claude (AI). Licensed under MIT OR Apache-2.0.

**This is a group of building blocks.** Planning means: what, in which order, by whom, done when. Tasks, workflows (runner, Gantt) and later schedules share these parts instead of each carrying its own copy.

## Status

Skeleton only: the workspace holds one empty crate (`crates/fs-modules-planning`, documentation only, no code) so that it builds; it goes away with the first real module. The first content is the island of the tasks program, moved unchanged (crates `fs-tasks-local` and `fs-tasks-testdata`, under their old names; renaming to `fs-todo` is a later step).

## Modules that will come

- `fs-dependency`: dependencies between items (RFC 9253 `FINISHTOSTART`, `DEPENDS-ON`), cycles, readiness.
- `fs-critical-path`: critical path and slack (CPM), built on `fs-dependency`.
- `fs-rank`: hand order of items (fractional rank).
- `fs-checklist`: checklists with their own order, built on `fs-rank`.
- `fs-progress`: progress states as a rule set with RFC 8984 defaults.
- `fs-claim`: who takes an item, and until when.
- `fs-todo` (data module): the to-do item as JSCalendar Task (RFC 8984) / VTODO (RFC 5545), with `fs-todo-testdata`.

## About the AI help

This repository is written with the help of Claude (Anthropic), an AI. The commits say so (`Co-Authored-By`). Every change is checked by tests and by a second review before it lands on `main`.

## Gates

`cargo fmt --check`, `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings -W clippy::pedantic`, `cargo test --workspace --all-features --locked`, and the coding rules of fs-release (`scripts/gates.sh`). CI: `.github/workflows/check.yml` (fs-release v0.0.10) and `windows.yml`.

Licence: MIT OR Apache-2.0.

## Auf Deutsch

FreeSynergy-Planungsmodule: Abhaengigkeiten (RFC 9253), kritischer Pfad, Handreihenfolge, Checklisten, Fortschrittszustaende, Ansprueche und das Aufgaben-Datenmodul (JSCalendar Task, RFC 8984; VTODO, RFC 5545). Mit Hilfe von Claude (KI) gebaut. Lizenz MIT OR Apache-2.0. Stand: nur das Geruest, der Arbeitsbereich ist leer; als Erstes kommt die Insel des Aufgabenprogramms unveraendert hierher.
