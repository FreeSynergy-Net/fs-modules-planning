# FreeSynergy planning modules

FreeSynergy planning modules: dependencies (RFC 9253), critical path, hand order, checklists, progress states, claims and the to-do data module (JSCalendar Task, RFC 8984; VTODO, RFC 5545). Built with help from Claude (AI). Licensed under MIT OR Apache-2.0.

**This is a group of building blocks.** Planning means: what, in which order, by whom, done when. Tasks, workflows (runner, Gantt) and later schedules share these parts instead of each carrying its own copy.

## Status

Latest tag: v0.0.2; the next release is 0.0.3.

Today this repository is ONE tasks provider plus ONE test kit, moved unchanged from [fs-tasks](https://github.com/FreeSynergy/fs-tasks) (main `ae7be9da0168ed2c05eeca4ce0ff48c4c7a4dae0`): `crates/fs-tasks-local` (provider of the `Tasks` trait `tasks_v1` over typed fs-objects v5 tables) and `crates/fs-tasks-testdata` (fake, conformance rules, sample data) with the type `Task`, under their old names. Both crates have version 0.0.1. All 150 files are byte for byte equal to the source (list of sha256: `docs/insel-umzug-sha256.txt`). Git pins are as in fs-tasks (fs-objects v0.0.2, fs-types = fs-interfaces v0.0.22, fs-jscalendar v0.0.2, fs-people rev 358a8145).

Readiness, rank, checklist, progress and claim exist as code only INSIDE the provider, bound to `tasks_v1`; none of them is a module of its own yet. Schedule and critical path do not exist at all, and the progress presets `rfc8984` and `freesynergy` are missing. The long rank test is `#[ignore]` and runs by hand only (`cargo test -p fs-tasks-local -- --ignored`), not in CI.

## Form source (tasks)

The service `fs-modules-planning` is the module `planning`: socket `planning.sock`, data in `modules/planning`, the one constant `fs_modules_planning_client::SOCKET_PROGRAM`. Next to `tasks.capnp` it serves `FormSource` (level 1) on the same socket: form `cards-in-columns@1.0` (`catalog/forms.json`), the three write commands `card.move` (`board`, `column`, `index`), `task.create` (`title`, `board`, `column`, `index`) and `task.delete` through `invoke` with text values, named by their capability key of the contract (the host binds them as `planning.<key>`); unknown, doubled or missing arguments are refused with the reason; `may` says what the provider supports (the core has no rights per person). `column.create`, `claim`, `release` and `note` are not commands yet: no service serves them, so they are not in `catalog/commands.json`. The file `catalog/commands.json` names the commands by the same keys the service reports (`task.create`, `card.move`, `board.open` ...), without prefix. It runs the heartbeat of fs-rpc (Unix). `catalog/commands.json` has the version `0.0.2` and the workspace the version 0.0.3: the first is the version of the command set (what the host reads in the file), the second the version of the crates and the tag. They are two numbers with two meanings and do not have to be equal; the command set starts at 0.0.1 and rises when a command changes. In 0.0.x every step may break. In 0.0.x darf jeder Schritt brechen.

## Modules that will come (targets, not present)

Decision (strategy 6e, F2): the renaming to `fs-todo` stays a later step. A bundle `fs-planning` is made only when at least two programs use the parts (ADR 0101 E2); until then the parts below are single modules.

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

`cargo fmt --check`, `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings -W clippy::pedantic`, `cargo test --workspace --all-features --locked`, and the coding rules of fs-release (`scripts/gates.sh`). CI: `.github/workflows/check.yml` (fs-release v0.0.11) and `windows.yml` (`_windows-check.yml@v0.0.11`).

Licence: MIT OR Apache-2.0.

## Auf Deutsch

FreeSynergy-Planungsmodule: Abhaengigkeiten (RFC 9253), kritischer Pfad, Handreihenfolge, Checklisten, Fortschrittszustaende, Ansprueche und das Aufgaben-Datenmodul (JSCalendar Task, RFC 8984; VTODO, RFC 5545). Mit Hilfe von Claude (KI) gebaut. Lizenz MIT OR Apache-2.0. Stand: Heute ist das Repository EIN Aufgaben-Provider (`fs-tasks-local`) plus EIN Testbausatz (`fs-tasks-testdata`) mit dem Typ Task, unveraendert aus fs-tasks verschoben; beide Crates haben die Version 0.0.1. Bereitschaft, Rang, Checkliste, Fortschritt und Anspruch gibt es als Code nur innerhalb des Providers, keine davon als eigenes Modul. Terminplan und kritischer Pfad fehlen ganz, ebenso die Fortschritts-Voreinstellungen rfc8984 und freesynergy. Neuester Tag: v0.0.2; das naechste Release ist 0.0.3. Die Umbenennung zu `fs-todo` bleibt ein spaeterer Schritt; ein Bausatz `fs-planning` entsteht erst, wenn mindestens zwei Programme die Teile nutzen (ADR 0101 E2).
