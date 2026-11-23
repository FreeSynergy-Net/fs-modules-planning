# Changelog

Version = tag.

## Unreleased

English

- The module is named `planning`: socket `planning.sock` (the old name `modules-planning.sock` is gone, no alias), one constant `SOCKET_PROGRAM` in the client, used by the service; data folder `modules/planning`.
- `FormSource` (level 1) next to `tasks.capnp` on the same socket: `hello`, `page` for `cards-in-columns@1.0`, `invoke` for `move` with text values; `catalog/forms.json`; `catalog/commands.json` version `0.0.1` with `claim`, `release`, `note` (listed, not served yet).
- Heartbeat of fs-rpc started with the service (Unix).
- Pins: fs-rpc v0.0.7, fs-interfaces v0.0.31.

Auf Deutsch

- Das Modul heisst `planning`: Socket `planning.sock` (der alte Name `modules-planning.sock` entfaellt ersatzlos), eine Konstante `SOCKET_PROGRAM` im Client, vom Dienst benutzt; Datenordner `modules/planning`.
- `FormSource` (Stufe 1) neben `tasks.capnp` auf demselben Socket: `hello`, `page` fuer `cards-in-columns@1.0`, `invoke` fuer `move` mit Textwerten; `catalog/forms.json`; `catalog/commands.json` Version `0.0.1` mit `claim`, `release`, `note` (gelistet, noch nicht bedient).
- Herzschlag von fs-rpc mit dem Dienst gestartet (Unix).
- Pins: fs-rpc v0.0.7, fs-interfaces v0.0.31.

## 0.0.3 - 2026-11-21

English

- Version 0.0.2 to 0.0.3 in `[workspace.package]`; no code change.
- Pins lifted: fs-interfaces v0.0.29 to v0.0.30 (fs-tasks-core, fs-modules-planning, fs-modules-planning-client) and fs-rpc v0.0.5 to v0.0.6 (fs-modules-planning, fs-modules-planning-client); lock updated.

Auf Deutsch

- Version 0.0.2 auf 0.0.3 in `[workspace.package]`; keine Code-Aenderung.
- Pins gehoben: fs-interfaces v0.0.29 auf v0.0.30 (fs-tasks-core, fs-modules-planning, fs-modules-planning-client) und fs-rpc v0.0.5 auf v0.0.6 (fs-modules-planning, fs-modules-planning-client); Lock nachgezogen.

## 0.0.2 - 2026-11-20

English

- Version 0.0.1 to 0.0.2 in `[workspace.package]` (all crates inherit it); no code change.
- New: the planning module service `fs-modules-planning` (one process per person, Unix socket through fs-rpc, data in `$XDG_DATA_HOME/freesynergy/modules/planning/`, the actor only from the connection) serving `tasks.capnp`: `Items` (create, get, getByUid, delete, list) and `Boards` (create, list, layout, editColumns with compare-and-set, moveCard) over the `Boards` trait of `fs-tasks-core`.
- New: `fs-modules-planning-client` (columns, add card, move card, add column, rename, limit, `can(operation)` from the reported capabilities, `ensure_service`) and the command catalog `catalog/commands.json` (command id to contract call).
- `fs-tasks-core`: `TaskRepo::as_actor` (a view written as another actor, one per connection).
- Transition: fs-interfaces main 4f5834db by `[patch]` (Boards methods; no tag yet). `fs-tasks-server` (built on `fs_interfaces::rpc`, gone after fs-interfaces 0.0.28) is excluded from the workspace and superseded.

Auf Deutsch

- Version 0.0.1 auf 0.0.2 in `[workspace.package]` (alle Crates erben sie); keine Code-Aenderung.
- Neu: der Dienst des Planungsmoduls `fs-modules-planning` (ein Prozess je Person, Unix-Socket ueber fs-rpc, Daten unter `$XDG_DATA_HOME/freesynergy/modules/planning/`, der Akteur nur aus der Verbindung) mit `tasks.capnp`: `Items` (create, get, getByUid, delete, list) und `Boards` (create, list, layout, editColumns mit Compare-and-Set, moveCard) ueber den `Boards`-Trait von `fs-tasks-core`.
- Neu: `fs-modules-planning-client` (Spalten, Karte anlegen, Karte verschieben, Spalte anlegen, umbenennen, Limit, `can(Operation)` aus den gemeldeten Faehigkeiten, `ensure_service`) und der Befehlskatalog `catalog/commands.json` (Befehls-ID zu Vertragsaufruf).
- `fs-tasks-core`: `TaskRepo::as_actor` (eine Sicht, die als anderer Akteur schreibt, je Verbindung eine).
- Uebergang: fs-interfaces main 4f5834db per `[patch]` (Boards-Methoden; noch kein Tag). `fs-tasks-server` (auf `fs_interfaces::rpc` gebaut, nach fs-interfaces 0.0.28 weg) ist aus dem Workspace ausgenommen und abgeloest.

## 0.0.1 - 2026-11-18

English

- What this repository is today: ONE tasks provider (`fs-tasks-local`, the `Tasks` trait `tasks_v1` over typed fs-objects v5 tables) plus ONE test kit (`fs-tasks-testdata`: fake, conformance rules, sample data) with the type `Task` (JSCalendar Task, RFC 8984).
- Readiness, rank, checklist, progress (state changes with the children rule) and claim exist as code ONLY inside the provider, bound to `tasks_v1`. None of them is a module of its own yet.
- Missing completely: schedule and critical path (no code). Missing: the progress presets `rfc8984` and `freesynergy`.
- Moved unchanged from fs-tasks main ae7be9da (150 files, sha256 equal to the source, `docs/insel-umzug-sha256.txt`). Git pins unchanged (fs-objects v0.0.2, fs-types = fs-interfaces v0.0.22, fs-jscalendar v0.0.2, fs-people rev 358a8145).
- Both crates carry the version 0.0.1 (`[workspace.package] version = "0.0.1"`, `version.workspace = true` in `crates/fs-tasks-local/Cargo.toml` and `crates/fs-tasks-testdata/Cargo.toml`).
- Licences MIT and Apache-2.0, README, CI (`check.yml` and `windows.yml` at fs-release v0.0.11). The shortened crate description of `fs-tasks-local` and a corrected comment on the ignored long rank test (it is run by hand, not by CI).
- Written with Claude (Anthropic).

Auf Deutsch

- Was dieses Repository heute ist: EIN Aufgaben-Provider (`fs-tasks-local`, der Trait `Tasks` `tasks_v1` ueber typisierte fs-objects-v5-Tabellen) plus EIN Testbausatz (`fs-tasks-testdata`: Fake, Konformitaetsregeln, Beispieldaten) mit dem Typ `Task` (JSCalendar Task, RFC 8984).
- Bereitschaft, Rang, Checkliste, Fortschritt (Zustandswechsel mit der Kinder-Regel) und Anspruch gibt es als Code NUR innerhalb des Providers, an `tasks_v1` gebunden. Keine davon ist schon ein eigenes Modul.
- Ganz fehlend: Terminplan und kritischer Pfad (kein Code). Fehlend: die Fortschritts-Voreinstellungen `rfc8984` und `freesynergy`.
- Unveraendert aus fs-tasks main ae7be9da verschoben (150 Dateien, sha256 gleich zur Quelle, `docs/insel-umzug-sha256.txt`). Git-Pins unveraendert (fs-objects v0.0.2, fs-types = fs-interfaces v0.0.22, fs-jscalendar v0.0.2, fs-people rev 358a8145).
- Beide Crates tragen die Version 0.0.1 (`[workspace.package] version = "0.0.1"`, `version.workspace = true` in `crates/fs-tasks-local/Cargo.toml` und `crates/fs-tasks-testdata/Cargo.toml`).
- Lizenzen MIT und Apache-2.0, README, CI (`check.yml` und `windows.yml` mit fs-release v0.0.11). Gekuerzte Crate-Beschreibung von `fs-tasks-local` und korrigierter Kommentar zum ignorierten langen Rang-Test (er laeuft von Hand, nicht in der CI).
- Geschrieben mit Hilfe von Claude (Anthropic).
