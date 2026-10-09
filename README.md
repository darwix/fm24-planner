# FM Squad Planner

A local Football Manager 2024 squad planner. Load a `.fm` save, arrange your squad on a 4-2-3-1 pitch, cover every position, and find replacements. Your save never leaves your computer.

Built on [FM SaveLens 24](https://github.com/MSalman5230/FM-SaveLens-24), whose Rust parser reads the save. This repository patches it to also read wage, contract end and value. It has no licence, so keep this repository private. Supports FM24 save format **24.3.0+0**. Not affiliated with Sports Interactive or SEGA.

## Features

- **Squad**: pitch with drag and drop, position depth panel, multi-position cover, add and remove players.
- **Find replacement / Add cover**: your own squad first, then outside players ranked by CA against the player replaced, tagged contract expiring or cheap.
- **Scouting**: preset lists (promising youngsters, contract expiring) and custom lists.
- **Your club** is detected from the save. Your arrangement is kept in the browser (local storage).

## Run

Needs Node.js 24.14+ and Rust (pinned in `rust-toolchain.toml`).

```sh
npm --prefix web ci
npm run build:web
cargo run --release --bin fm-savelens-24-server -- --no-open --port 4242
```

Open [localhost:4242](http://localhost:4242). For frontend hot reload, run `npm run dev` in `web/` and open [localhost:5173](http://localhost:5173); it proxies `/api` to the server on :4242.

## Test

```sh
cd web && npm test && npm run typecheck && npm run format:check
cargo test --release --locked
```

A pre-commit hook runs Prettier on `web/`, and the PR workflow checks format, tests and build.

## Layout

- `web/`: React 18 + TypeScript app (react-router, recoil, swr, axios). Builds to `web/dist/client`, which the server embeds.
- `native/`: Rust parser and API server (`native/parser/contract.rs` reads wage, contract end and value).
- `mock/`: the chosen HTML design (`a-board.html`).
- `CLAUDE.md`: project notes and decisions.
