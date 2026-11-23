# Changelog

Version = tag.

## Unreleased

English

- SW-6, write path: `catalog/commands.json` (version `0.0.2`) names each command by its capability key of the contract (`card.move`, `task.create`, `task.delete`; no `id`, no prefix: the host derives `planning.<key>`) and lists its `params` (`path`, `type`, `required`). `FormSource.invoke` takes exactly these keys on the wire (`planning.move` and `planning.card.move` are `form-source-unknown-command`). `task.create` makes the task and puts its card into `column` of `board` at `index`; `task.delete` deletes a task; `card.move` as before. The answer names the card in `result` (`record`, `text`: `card-moved`, `task-created`, `task-deleted`).
- `index` counts among the cards as the layout shows them (the core counts only the cards placed by hand): the order of every column is written as shown before the card is placed (checked edit; a board changed in between is a conflict). Test with a full column.
- Arguments: unknown, doubled or missing ones are `form-source-bad-arguments` with the reason (before: two own kinds); an unknown task is `form-source-unknown-record` (404), a uid of 0 or more than 255 bytes `bad-arguments`; a refusal of the core is `form-source-rejected` (422) or `form-source-conflict` (409), never `internal`; `record` of `task.create` must be empty.
- `may`: per record `card.move`, `task.delete`, and in `RecordPage.may` `task.create`, as the provider reports them (`TaskItems::capabilities`); the core has no rights per person, so nothing more is claimed. `invoke` refuses (`form-source-forbidden`, 403) what `may` does not name.
- `catalog/forms.json`: `move` is `card.move`. The client catalog is keyed by the capability key and reads `params`; a command with an `id` is refused.
- Re-pin on the FreeSynergy-Net URLs: fs-rpc v0.0.8, fs-interfaces v0.0.34, fs-objects v0.0.10, fs-jscalendar v0.0.7; fs-release references on 9eed5c95 (v0.0.18). No version raise (comes with the closing bundle).
- Fix after review (NEIN): the capability ids the service reports are the ids of the contract again (`task.create`, `card.move`, `board.open` ...); only `catalog/commands.json` keeps the prefix `planning.` (the client maps it with `Command::contract_capability`). Test: the service reports exactly the contract ids.
- `catalog/commands.json` lists only `planning.move`; `planning.create`, `planning.column.create`, `planning.delete` are out (no service behind them, they come with the write path).
- `catalog/forms.json`: `facts` has 4 fields (`progress` out), as `cards-in-columns@1.0` allows (0..4).
- `catalog/forms.json`: `due` is `date-time` (was `datetime`, a word the host does not know); test: every field type is a word of the host reader.
- `invoke` refuses unknown and doubled arguments (`form-source-unknown-argument`, `form-source-duplicate-argument`), a missing or too long `record` and an unknown task; a cancelled run is `form-source-internal`, not bad-arguments; a board name that is not unique is refused (the id wins).
- Priority 0 (RFC 8984: undefined) is no fact. Tests for `index`, `may` and the paging cursor.
- Version: `catalog/commands.json` `0.0.1` is the version of the command set, the workspace `0.0.3` the version of the crates (and the tag). Not a step back, two numbers.
- The module is named `planning`: socket `planning.sock` (the old name `modules-planning.sock` is gone, no alias), one constant `SOCKET_PROGRAM` in the client, used by the service; data folder `modules/planning`.
- `FormSource` (level 1) next to `tasks.capnp` on the same socket: `hello`, `page` for `cards-in-columns@1.0`, `invoke` for `planning.move` with text values; `catalog/forms.json`; `page` also sends `priority`, `participants`, `keywords`, `description` of the task when it has them.
- `catalog/commands.json` (version `0.0.1`) fulfils the duties of the host: `contract` = `module` = `planning`, `scope` `own-contract`, two-part command ids (`planning.move`), capabilities with the prefix `planning.`, `access` and `help` (en, de) per command. `claim`, `release`, `note` are NOT in it (no service behind them).
- Heartbeat of fs-rpc started with the service (Unix).
- Pins: fs-rpc v0.0.7, fs-interfaces v0.0.31.

Auf Deutsch

- SW-6, Schreibweg: `catalog/commands.json` (Version `0.0.2`) nennt jeden Befehl mit dem Faehigkeitsschluessel des Vertrags (`card.move`, `task.create`, `task.delete`; kein `id`, kein Praefix: der Host leitet `planning.<Schluessel>` ab) und fuehrt seine `params` (`path`, `type`, `required`). `FormSource.invoke` nimmt genau diese Schluessel auf dem Draht (`planning.move` und `planning.card.move` sind `form-source-unknown-command`). `task.create` legt die Aufgabe an und stellt ihre Karte in `column` von `board` an `index`; `task.delete` loescht eine Aufgabe; `card.move` wie bisher. Die Antwort nennt die Karte in `result` (`record`, `text`: `card-moved`, `task-created`, `task-deleted`).
- `index` zaehlt unter den Karten, wie das Layout sie zeigt (der Kern zaehlt nur die von Hand platzierten): vor dem Platzieren wird die Reihenfolge jeder Spalte so geschrieben, wie sie zu sehen ist (geprueftes Aendern; ein dazwischen geaendertes Board ist ein Konflikt). Test mit voller Spalte.
- Argumente: unbekannte, doppelte oder fehlende sind `form-source-bad-arguments` mit Grund (vorher zwei eigene Arten); eine unbekannte Aufgabe ist `form-source-unknown-record` (404), eine uid mit 0 oder mehr als 255 Bytes `bad-arguments`; eine Ablehnung des Kerns ist `form-source-rejected` (422) oder `form-source-conflict` (409), nie `internal`; `record` von `task.create` muss leer sein.
- `may`: je Datensatz `card.move`, `task.delete`, in `RecordPage.may` `task.create`, so wie der Anbieter sie meldet (`TaskItems::capabilities`); der Kern kennt keine Rechte je Person, mehr wird nicht behauptet. `invoke` lehnt ab (`form-source-forbidden`, 403), was `may` nicht nennt.
- `catalog/forms.json`: `move` ist `card.move`. Der Katalog im Client ist nach dem Faehigkeitsschluessel geordnet und liest `params`; ein Befehl mit `id` wird abgelehnt.
- Umpinnen auf die FreeSynergy-Net-Adressen: fs-rpc v0.0.8, fs-interfaces v0.0.34, fs-objects v0.0.10, fs-jscalendar v0.0.7; fs-release-Verweise auf 9eed5c95 (v0.0.18). Keine Versionshebung (kommt mit dem Schlussbuendel).
- Korrektur nach Review (NEIN): Die Faehigkeiten, die der Dienst meldet, sind wieder die ids des Vertrags (`task.create`, `card.move`, `board.open` ...); nur `catalog/commands.json` behaelt den Praefix `planning.` (der Client uebersetzt mit `Command::contract_capability`). Test: Der Dienst meldet genau die Vertrags-ids.
- `catalog/commands.json` fuehrt nur noch `planning.move`; `planning.create`, `planning.column.create`, `planning.delete` sind heraus (kein Dienst dahinter, sie kommen mit dem Schreibweg).
- `catalog/forms.json`: `facts` hat 4 Felder (`progress` heraus), wie `cards-in-columns@1.0` erlaubt (0..4).
- `catalog/forms.json`: `due` ist `date-time` (vorher `datetime`, ein Wort, das der Host nicht kennt); Test: jede Feldart ist ein Wort des Host-Lesers.
- `invoke` lehnt unbekannte und doppelte Argumente ab (`form-source-unknown-argument`, `form-source-duplicate-argument`), ebenso fehlendes oder zu langes `record` und unbekannte Aufgabe; ein abgebrochener Lauf ist `form-source-internal`, nicht bad-arguments; ein nicht eindeutiger Boardname wird abgelehnt (die id gilt zuerst).
- Prioritaet 0 (RFC 8984: undefiniert) ist kein Fakt. Tests fuer `index`, `may` und den Seiten-Cursor.
- Version: `catalog/commands.json` `0.0.1` ist die Version des Befehlssatzes, der Workspace `0.0.3` die Version der Crates (und des Tags). Kein Rueckschritt, zwei Zahlen.
- Das Modul heisst `planning`: Socket `planning.sock` (der alte Name `modules-planning.sock` entfaellt ersatzlos), eine Konstante `SOCKET_PROGRAM` im Client, vom Dienst benutzt; Datenordner `modules/planning`.
- `FormSource` (Stufe 1) neben `tasks.capnp` auf demselben Socket: `hello`, `page` fuer `cards-in-columns@1.0`, `invoke` fuer `planning.move` mit Textwerten; `catalog/forms.json`; `page` sendet zusaetzlich `priority`, `participants`, `keywords`, `description` der Aufgabe, wenn sie sie hat.
- `catalog/commands.json` (Version `0.0.1`) erfuellt die Pflichten des Hosts: `contract` = `module` = `planning`, `scope` `own-contract`, zweiteilige Befehls-ids (`planning.move`), Faehigkeiten mit Praefix `planning.`, `access` und `help` (en, de) je Befehl. `claim`, `release`, `note` stehen NICHT darin (kein Dienst dahinter).
- Client-Tests nachgezogen (ids `planning.*`).
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
