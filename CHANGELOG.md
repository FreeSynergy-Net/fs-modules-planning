# Changelog

Version = tag.

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
