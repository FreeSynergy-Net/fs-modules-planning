# Changelog

Version = tag.

## Unreleased

English

- Image-2 follow-up: `task.create` without `board` (the form `lane` is optional on create) takes the only board; none or several boards is `form-source-bad-arguments` (400) naming the reason; `board` is optional in `commands.json`.
- Image-2 follow-up: `due` is sent as an RFC 3339 instant with offset (`...Z`, from `due` + `timeZone`, else the floating zone UTC), no longer as naive text; `column-label` (column name) and `progress` (percent) are mapped in `forms.json`. The form cannot carry empty columns (card).
- Stage 1, Package M3: the place of a card is never a number. `Board.order` (a list per column) is replaced by `Board.place` (per card: column + rank key as text, made by the workspace crate `fs-rank`); no migration (an old board with `order` is read without it). The order in a column is the sort by `(key, uid)`; two equal keys are decided by the uid, nothing is renumbered. `card.move` and `task.create` take `board`, `column` and `after`/`before` (record ids of the neighbour cards; empty = none) instead of `index`; if the neighbours are no longer neighbours, `after` is the anchor, else `before`, else the end (nothing is refused, nothing lost). Cards before the place that never had one get a place where they stand now; the cards behind keep following their `progress`. A caller that still sends `order` or `index` gets `form-source-bad-arguments` (400) with the reason. `Boards.moveCard(index)` of the contract stays (index among the placed cards, now via keys); the contract `order` lists are derived from `place` and written back unchanged without new keys.
- `catalog/forms.json`: `cards-in-columns` `1.2` (the file keeps `x.y` until the host takes `x.y.z`); `map` has `after`, `before` (`order` stays and is read only: its value `index` is the position 0..n of the card in its column). `page` takes `@1.2`, `@1.1`, `@1.0` in both forms (`x.y`, `x.y.z`) via `fs_interfaces::forms::parse_key`; other minors and majors are `form-source-unknown-form`. `catalog/commands.json` `0.0.3`: params `after`, `before` (`reference`) instead of `index`.
- `blob` says `form-source-unsupported` (501), no longer a URL of its own.
- Pins: fs-interfaces v0.1.0 (tag with `version`), fs-rpc v0.1.3 (tag with `version`; it has fs-interfaces v0.1.0, so one source per library; the interim rev pin of the first M3 head is gone).
- WP2: `tests/conformance.rs` runs the suite `fs-module` (conformance of the contract `module_v1`, fs-interfaces v0.1.0) against the service in this process (temporary store), and T21 (move by neighbours puts the card there and `order` counts 0, 1, 2; `index`/`order` are refused with 400).
- New crate `fs-rank` (publish = false): thin adapter around `fracindex =0.4.1` (feature `jitter`). `Key::first/after/before/between/place`, text form `to_text`/`from_text` (lowercase hex, sorts like the key; test), `Error::NotOrdered` for lower >= upper. Used by `Board.place` (M3, below).
- SW-6, write path: `catalog/commands.json` (version `0.0.2`) names each command by its capability key of the contract (`card.move`, `task.create`, `task.delete`; no `id`, no prefix: the host derives `planning.<key>`) and lists its `params` (`path`, `type`, `required`). `FormSource.invoke` takes exactly these keys on the wire (`planning.move` and `planning.card.move` are `form-source-unknown-command`). `task.create` makes the task and puts its card into `column` of `board` at `index`; `task.delete` deletes a task; `card.move` as before. The answer names the card in `result` (`record`, `text`: `card-moved`, `task-created`, `task-deleted`).
- `index` counts among the cards as the layout shows them (the core counts only the cards placed by hand). So exactly the shown cards BEFORE the insertion point of the TARGET column are written into the order of the board (nothing moves on the screen); other columns are not touched and the cards behind the insertion point keep following their `progress` (a task that is completed elsewhere still goes to `done`). Pinning those cards is unavoidable for a place among cards that have none. A board changed in between is a conflict (checked edit). Tests with a full column and with a change of `progress` after a move.
- Arguments: unknown, doubled or missing ones are `form-source-bad-arguments` with the reason (before: two own kinds); an unknown task is `form-source-unknown-record` (404), a uid of 0 or more than 255 bytes `bad-arguments`; a refusal of the core is `form-source-rejected` (422) or `form-source-conflict` (409), never `internal`; `record` of `task.create` must be empty.
- `may`: per record `card.move`, `task.delete`, and in `RecordPage.may` `task.create`, as the provider reports them (`TaskItems::capabilities`); the core has no rights per person, so nothing more is claimed. `invoke` refuses (`form-source-forbidden`, 403) what `may` does not name.
- `catalog/forms.json`: version `1.1` (`cards-in-columns@1.1`, fs-interfaces v0.0.34 `forms.txt`) with `map` for `order` (`index`), `move` (`card.move`), `create` (`task.create`), `remove` (`task.delete`), so the host finds the `writes` of the gestures; `progress` is not bound (a number channel in 1.1, the module sends text). `page` takes `cards-in-columns@1.1` and, as its subset, `@1.0` (`FORM_CARDS`, `FORM_CARDS_1_0`). The client catalog is keyed by the capability key and reads `params`; a command with an `id` is refused.
- `task.create`: if the placing fails after the task was made (conflict, column or board gone in between), the task is deleted again and the placing error is the answer: a failed command leaves nothing behind, a retry does not make it twice.
- `may` has a reason in the code: only the owner is admitted at the socket (fs-rpc: Unix mode 0600, Windows `only_me` and no remote clients), the owner may do all with his data, and `invoke` gives no right that `Tasks` on the same socket does not give already; `may` is what the provider supports. Rights per person come later. The kind for a command the provider cannot do stays `form-source-forbidden` (403): `formsource.capnp` names no kinds for `invoke` and the host (SW-5) tells them apart only later, so no new kind `unsupported` (501) is started.
- Re-pin on the FreeSynergy-Net URLs: fs-rpc v0.0.8, fs-interfaces v0.0.34, fs-objects v0.0.10, fs-jscalendar v0.0.7; fs-release references on 9eed5c95 (v0.0.18). No version raise (comes with the closing bundle).
- `catalog/forms.json`: `facts` has 4 fields (`progress` out), as `cards-in-columns@1.0` and `@1.1` allows (0..4).
- `catalog/forms.json`: `due` is `date-time` (was `datetime`, a word the host does not know); test: every field type is a word of the host reader.
- Priority 0 (RFC 8984: undefined) is no fact. Tests for `index`, `may` and the paging cursor.
- Version: `catalog/commands.json` `0.0.2` is the version of the command set, the workspace `0.0.3` the version of the crates (and the tag). Not a step back, two numbers.
- The module is named `planning`: socket `planning.sock` (the old name `modules-planning.sock` is gone, no alias), one constant `SOCKET_PROGRAM` in the client, used by the service; data folder `modules/planning`.
- `FormSource` (level 1) next to `tasks.capnp` on the same socket: `hello`, `page` for `cards-in-columns@1.1` (and `@1.0`), `invoke` for `card.move`, `task.create`, `task.delete` with text values; `catalog/forms.json`; `page` also sends `priority`, `participants`, `keywords`, `description` of the task when it has them.
- `catalog/commands.json` (version `0.0.2`) fulfils the duties of the host: `contract` = `module` = `planning`, `scope` `own-contract`, commands by the capability key of the contract (no `id`, no prefix: the host derives `planning.<key>`), `access` and `help` (en, de) per command. `claim`, `release`, `note` are NOT in it (no service behind them).
- Heartbeat of fs-rpc started with the service (Unix).

Auf Deutsch

- Bild-2-Nachzug: `task.create` ohne `board` nimmt das einzige Board; kein oder mehrere Boards ergibt `form-source-bad-arguments` (400) mit Grund.
- Bild-2-Nachzug: `due` geht als RFC-3339-Zeitpunkt mit Offset (`...Z`, aus `due` + `timeZone`, sonst Schwebezone UTC) hinaus, nicht mehr als naiver Text; `column-label` (Spaltenname) und `progress` (Prozent) sind in `forms.json` abgebildet. Leere Spalten kann die Form nicht tragen (Karte).
- Stufe 1, Paket M3: Die Stelle einer Karte ist nie eine Zahl. `Board.order` (Liste je Spalte) wird durch `Board.place` ersetzt (je Karte: Spalte + Rangschluessel als Text, erzeugt vom Workspace-Crate `fs-rank`); keine Migration (ein altes Board mit `order` wird ohne gelesen). Die Reihenfolge in einer Spalte ist die Sortierung nach `(key, uid)`; gleiche Schluessel entscheidet die uid, nichts wird neu nummeriert. `card.move` und `task.create` nehmen `board`, `column` und `after`/`before` (Datensatz-Ids der Nachbarkarten; leer = keine) statt `index`; sind die Nachbarn keine Nachbarn mehr, ist `after` der Anker, sonst `before`, sonst das Ende (nichts wird abgelehnt, nichts geht verloren). Karten vor der Stelle ohne eigene Stelle bekommen eine, wo sie jetzt stehen; die dahinter folgen weiter ihrem `progress`. Wer noch `order` oder `index` schickt, bekommt `form-source-bad-arguments` (400) mit Grund. `Boards.moveCard(index)` des Vertrags bleibt (Index unter den platzierten Karten, jetzt ueber Schluessel); die `order`-Listen des Vertrags werden aus `place` abgeleitet und unveraendert zurueckgeschrieben, ohne neue Schluessel.
- `catalog/forms.json`: `cards-in-columns` `1.2` (die Datei behaelt `x.y`, bis der Host `x.y.z` nimmt); `map` hat `after`, `before` (`order` bleibt und wird nur gelesen: sein Wert `index` ist die Stelle 0..n der Karte in ihrer Spalte). `page` nimmt `@1.2`, `@1.1`, `@1.0` in beiden Schreibweisen (`x.y`, `x.y.z`) ueber `fs_interfaces::forms::parse_key`; andere Minor und Major sind `form-source-unknown-form`. `catalog/commands.json` `0.0.3`: Parameter `after`, `before` (`reference`) statt `index`.
- `blob` sagt `form-source-unsupported` (501), keine eigene URL mehr.
- Pins: fs-interfaces v0.1.0 (Tag mit `version`), fs-rpc v0.1.3 (Tag mit `version`; hat fs-interfaces v0.1.0, also eine Quelle je Bibliothek; der vorlaeufige rev-Pin des ersten M3-Kopfs entfaellt).
- WP2: `tests/conformance.rs` laesst die Suite `fs-module` (Konformitaet des Vertrags `module_v1`, fs-interfaces v0.1.0) gegen den Dienst im selben Prozess laufen (temporaerer Speicher), dazu T21 (move per Nachbarn setzt die Karte dorthin, `order` zaehlt 0, 1, 2; `index`/`order` werden mit 400 abgelehnt).
- SW-6, Schreibweg: `catalog/commands.json` (Version `0.0.2`) nennt jeden Befehl mit dem Faehigkeitsschluessel des Vertrags (`card.move`, `task.create`, `task.delete`; kein `id`, kein Praefix: der Host leitet `planning.<Schluessel>` ab) und fuehrt seine `params` (`path`, `type`, `required`). `FormSource.invoke` nimmt genau diese Schluessel auf dem Draht (`planning.move` und `planning.card.move` sind `form-source-unknown-command`). `task.create` legt die Aufgabe an und stellt ihre Karte in `column` von `board` an `index`; `task.delete` loescht eine Aufgabe; `card.move` wie bisher. Die Antwort nennt die Karte in `result` (`record`, `text`: `card-moved`, `task-created`, `task-deleted`).
- `index` zaehlt unter den Karten, wie das Layout sie zeigt (der Kern zaehlt nur die von Hand platzierten). Darum werden genau die gezeigten Karten VOR der Einfuegestelle der ZIELspalte in die Reihenfolge des Boards geschrieben (am Schirm bewegt sich nichts); andere Spalten bleiben unberuehrt, und die Karten hinter der Einfuegestelle folgen weiter ihrem `progress` (eine anderswo erledigte Aufgabe wandert weiter nach `done`). Diese Karten festzuhalten ist fuer einen Platz unter Karten ohne Platz unvermeidlich. Ein dazwischen geaendertes Board ist ein Konflikt (geprueftes Aendern). Tests mit voller Spalte und mit einer Aenderung von `progress` nach dem Verschieben.
- Argumente: unbekannte, doppelte oder fehlende sind `form-source-bad-arguments` mit Grund (vorher zwei eigene Arten); eine unbekannte Aufgabe ist `form-source-unknown-record` (404), eine uid mit 0 oder mehr als 255 Bytes `bad-arguments`; eine Ablehnung des Kerns ist `form-source-rejected` (422) oder `form-source-conflict` (409), nie `internal`; `record` von `task.create` muss leer sein.
- `may`: je Datensatz `card.move`, `task.delete`, in `RecordPage.may` `task.create`, so wie der Anbieter sie meldet (`TaskItems::capabilities`); der Kern kennt keine Rechte je Person, mehr wird nicht behauptet. `invoke` lehnt ab (`form-source-forbidden`, 403), was `may` nicht nennt.
- `catalog/forms.json`: Version `1.1` (`cards-in-columns@1.1`, fs-interfaces v0.0.34 `forms.txt`) mit `map` fuer `order` (`index`), `move` (`card.move`), `create` (`task.create`), `remove` (`task.delete`), damit der Host die `writes` der Gesten findet; `progress` ist nicht gebunden (in 1.1 ein Zahlenkanal, das Modul sendet Text). `page` nimmt `cards-in-columns@1.1` und als Teilmenge `@1.0` (`FORM_CARDS`, `FORM_CARDS_1_0`). Der Katalog im Client ist nach dem Faehigkeitsschluessel geordnet und liest `params`; ein Befehl mit `id` wird abgelehnt.
- `task.create`: scheitert das Platzieren, nachdem die Aufgabe angelegt wurde (Konflikt, Spalte oder Board dazwischen weg), wird die Aufgabe wieder geloescht und der Fehler des Platzierens ist die Antwort: ein gescheiterter Befehl hinterlaesst nichts, ein Wiederholen legt sie nicht doppelt an.
- `may` hat die Begruendung im Code: am Socket wird nur der Eigentuemer zugelassen (fs-rpc: Unix Modus 0600, Windows `only_me` und keine fernen Clients), der Eigentuemer darf an seinen Daten alles, und `invoke` gibt kein Recht, das `Tasks` auf demselben Socket nicht schon gibt; `may` ist, was der Anbieter kann. Rechte je Person kommen spaeter. Die Art fuer einen Befehl, den der Anbieter nicht kann, bleibt `form-source-forbidden` (403): `formsource.capnp` nennt fuer `invoke` keine Arten und der Host (SW-5) unterscheidet sie erst spaeter, darum wird keine neue Art `unsupported` (501) begonnen.
- Umpinnen auf die FreeSynergy-Net-Adressen: fs-rpc v0.0.8, fs-interfaces v0.0.34, fs-objects v0.0.10, fs-jscalendar v0.0.7; fs-release-Verweise auf 9eed5c95 (v0.0.18). Keine Versionshebung (kommt mit dem Schlussbuendel).
- `catalog/forms.json`: `facts` hat 4 Felder (`progress` heraus), wie `cards-in-columns@1.0` und `@1.1` erlaubt (0..4).
- `catalog/forms.json`: `due` ist `date-time` (vorher `datetime`, ein Wort, das der Host nicht kennt); Test: jede Feldart ist ein Wort des Host-Lesers.
- Prioritaet 0 (RFC 8984: undefiniert) ist kein Fakt. Tests fuer `index`, `may` und den Seiten-Cursor.
- Version: `catalog/commands.json` `0.0.2` ist die Version des Befehlssatzes, der Workspace `0.0.3` die Version der Crates (und des Tags). Kein Rueckschritt, zwei Zahlen.
- Das Modul heisst `planning`: Socket `planning.sock` (der alte Name `modules-planning.sock` entfaellt ersatzlos), eine Konstante `SOCKET_PROGRAM` im Client, vom Dienst benutzt; Datenordner `modules/planning`.
- `FormSource` (Stufe 1) neben `tasks.capnp` auf demselben Socket: `hello`, `page` fuer `cards-in-columns@1.1` (und `@1.0`), `invoke` fuer `card.move`, `task.create`, `task.delete` mit Textwerten; `catalog/forms.json`; `page` sendet zusaetzlich `priority`, `participants`, `keywords`, `description` der Aufgabe, wenn sie sie hat.
- `catalog/commands.json` (Version `0.0.2`) erfuellt die Pflichten des Hosts: `contract` = `module` = `planning`, `scope` `own-contract`, Befehle mit dem Faehigkeitsschluessel des Vertrags (kein `id`, kein Praefix: der Host leitet `planning.<Schluessel>` ab), `access` und `help` (en, de) je Befehl. `claim`, `release`, `note` stehen NICHT darin (kein Dienst dahinter).
- Herzschlag von fs-rpc mit dem Dienst gestartet (Unix).

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
