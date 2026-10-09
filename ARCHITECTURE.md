# Architecture

The Rust backend (the repo root) follows the layout of
[Thodin/axum-clean-architecture](https://github.com/Thodin/axum-clean-architecture):
Clean Architecture applied to an axum project, adapted where that was more practical.

Dependencies only point inwards:

```
infra ──▶ adapters ──▶ application ──▶ domain
```

`domain` knows nothing about the other layers. `application` knows only `domain`, `utils` and std.
`adapters` implement the ports `application` declares. `infra` wires it all together.

## Layout

```
src/
├── main.rs                 parse args, start the server, wait for Ctrl+C
├── lib.rs                  module declarations and re-exports
├── domain/                 what the app is about, no I/O
│   ├── entities/           Player, ParsedSave, Job, Settings, SnapshotId, PlayerFilter, ...
│   ├── catalog.rs          attributes and positions every player is rated on
│   └── countries.rs        nation id to name
├── application/            what the app does
│   ├── app_error.rs        Error and Result, shared by every layer
│   └── use_cases/
│       ├── ports.rs        traits the adapters implement
│       ├── saves.rs        SaveUseCase: list saves, snapshot metadata
│       ├── imports.rs      SaveUseCase: start, run, cancel an import
│       ├── players.rs      PlayerUseCase: search and read players
│       ├── settings.rs     SettingsUseCase: save folder
│       ├── session.rs      in-memory state: settings, import jobs, shutdown flag
│       └── snapshot_builder.rs   derived columns and metadata of a snapshot
├── adapters/
│   ├── http/               axum: routes, dto, error mapping, AppState
│   ├── persistence/        SQLite snapshots, JSON settings, files on disk, rusqlite/serde_json errors
│   └── parser/             reads the .fm save format, little-endian byte readers
├── infra/
│   ├── setup.rs            init_app_state: builds adapters and use cases
│   ├── app.rs              router with the host and origin guard
│   ├── data_dir.rs         data directory, instance lock, leftover cleanup
│   └── config.rs           command-line arguments and defaults
├── utils/                  helpers used by more than one layer: cancel (CancelFlag), files, text
└── assets/                 JSON catalogs (attributes, nations) embedded by domain
```

`mod.rs` files hold only module declarations and re-exports. A helper used by one layer lives in
that layer, not in `utils/`.

## Domain

Plain data and the rules that belong to it, with no framework or I/O.

- `Player`, `ParsedSave`, `Club`: what the parser reads from a save.
- `PlayerFilter`: a validated search, with `Range`, `ClubFilter`, `PositionFilter`, `SortKey`.
- `PlayerSummary`, `PlayerPage`, `PlayerDetail`: what a search or a player page returns.
- `NewSnapshot`, `PlayerRecord`: an import ready to store, every derived value worked out.
- `SnapshotId`: a 64-character hex digest, validated once when parsed.
- `SnapshotMeta`: the summary stored with a snapshot (source file, game date, nations, clubs).
- `Job`, `Settings`, `SaveFile`, `SourceFile`.

## Application

Use cases hold the logic and talk to the outside only through ports.

| Use case         | Does                                                          |
| ---------------- | ------------------------------------------------------------- |
| `SaveUseCase`    | lists saves, reads snapshot metadata, runs and cancels imports |
| `PlayerUseCase`  | searches players, reads one player                            |
| `SettingsUseCase`| reads and changes the save folder                             |

Ports (`use_cases/ports.rs`):

| Port                 | Implemented by                  |
| -------------------- | ------------------------------- |
| `SaveReader`         | `parser::reader::FmSaveReader`  |
| `SaveFiles`          | `persistence::FsSaveFiles`      |
| `SettingsRepository` | `persistence::JsonSettingsRepository` |
| `SnapshotRepository` | `persistence::SqliteSnapshotRepository` |
| `PlayerRepository`   | `persistence::SqlitePlayerRepository` |

`Session` is the only runtime state: the current settings, the import jobs and the shutdown
flag. The save and settings use cases share it behind an `Arc`.

## Adapters

### http

- `routes/`: one file per resource, each with `routes() -> Router<Shared>`, merged in
  `routes/router.rs`. Handlers are thin: extract, call a use case inside `blocking`, return.
- `dto/`: request and response shapes, and `player_query::parse_filter`, which turns a search
  query string into a `PlayerFilter` (`query_from_body` turns the POST body into that string).
- `error.rs`: `ApiError` maps the application `Error` to a status code and JSON body.
- `app_state.rs`: `AppState` holds the three use cases, `Shared = Arc<AppState>`.
- `response.rs`: `reply` (JSON with no-store headers) and `blocking` (runs a use case off the
  async runtime). `json_body.rs`: `JsonBody`, a JSON extractor with API errors.

### persistence

Plain lookups and writes, no decisions. `error.rs` maps rusqlite and serde_json errors to `Error`. Each repository opens its SQLite file per request
(read-only for reads); there is one file per imported save.

- Searches translate a `PlayerFilter` into SQL.
- Imports are written as `<job>.partial.sqlite` and published by rename.
- Settings live in `settings.json`, written through a temporary file.

### parser

Reads the `.fm` save. `save::parse_save` only orchestrates; each step has its own module:

- `archive`: the container (header, zstd index, members).
- `meta`: version check, in-game date, managed club from the save summary.
- `records/`: raw records in `game_db.dat`, one module per kind, each opening with its byte
  layout: `names`, `people`, `abilities` (and linking them to people), `clubs`, `teams`,
  `contract`. `scan` holds the byte-by-byte scan they share.
- `player`: each player's club, then the domain `Player` from the linked records.
- `report`: the checks that reject a bad import, and the diagnostics.

Bump `PARSER_VERSION` whenever the parser output or the snapshot schema changes, or re-import
reuses the cached snapshot.

## Infra

- `setup::init_app_state(data)` opens the data directory, builds the concrete adapters, injects
  them into the use cases and returns the `AppState`.
- `main` parses the arguments, binds the port, serves, and shuts down on Ctrl+C. Tests do the same
  in `tests/support::start`.
- `app::create_app` merges the route modules and adds the host and origin guard. The server is
  API-only; the React app in `web/` is served by Vite.

## Request flow

```
GET /api/snapshots/{id}/players?caMin=150
  routes::players::search_players     extract path and query
  dto::player_query::parse_filter     query string -> PlayerFilter (validation)
  PlayerUseCase::search_players       SnapshotId::parse, call the port
  SqlitePlayerRepository::search      PlayerFilter -> SQL -> PlayerPage
  reply                               JSON with no-store headers
```

## Adding an endpoint

1. Add the entity or filter to `domain/entities` and export it from `mod.rs`.
2. Add a use case method, and a port method if it needs new I/O.
3. Implement the port in `adapters/persistence` (or `parser`).
4. Add the route in `adapters/http/routes` and register it in `routes/router.rs`.
5. Add a test in `tests/`, run `cargo test --release`.
