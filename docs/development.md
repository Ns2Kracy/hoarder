# Hoarder Development

Hoarder is a Rust binary with an Axum API, SQLite persistence, OpenDAL source connectors, a multi-source one-way sync engine, and a Svelte/Vite management console.

## Product And Architecture Docs

- [Product PRD](prd.md): product positioning, MVP scope, user journeys, success metrics, and roadmap.
- [Architecture](architecture.md): current technical architecture, module boundaries, data model, API surface, and extension points.
- [Flows](flows.md): product and technical flows for source setup, job runs, source-to-vault decisions, vault writes, scheduler, errors, and settings.

## Prerequisites

- Rust 2024 toolchain
- Bun
- SQLite support through SeaORM/sqlx

## Build Order

The release binary embeds the built frontend from `web/dist`. Build the frontend before compiling Rust when you want the packaged UI to match the latest web source:

```bash
cd web
bun install
bun run build
cd ..
cargo build --release
```

During Rust compilation, `src/assets.rs` embeds the current contents of `web/dist` into the binary. Rebuild the frontend after changing files under `web/`.

## Verification

Run the strict Clippy gate before merging Rust changes:

```bash
cargo clippy --all-targets --all-features -- -D warnings
```

Run the backend test suite:

```bash
cargo test
```

Run the frontend checks and build:

```bash
cd web
bun run verify
```

Run the release packaging check:

```bash
cd web
bun run build
cd ..
cargo build --release
```

## Local Run

Start the packaged app on the default loopback address:

```bash
cargo run -- serve
```

Open `http://127.0.0.1:4761`. API routes are available under `/api/*`; frontend routes are served from the embedded app shell.

Override the server address:

```bash
cargo run -- serve --addr 127.0.0.1:4762
```

Sync the SQLite schema for the configured database:

```bash
cargo run -- db sync
```

Create and run a local filesystem workflow from the CLI:

```bash
cargo run -- source add --name docs --service fs --root ./docs
cargo run -- source list
cargo run -- source test --id 1
cargo run -- job add --source-id 1 --name docs --interval 300
cargo run -- job list
cargo run -- sync run --job-id 1
cargo run -- sync status
```

Create app connector sources through the generic JSON config path:

```bash
cargo run -- source add --name notion --config-json '{"kind":"notion","token":"secret","dataSourceId":"..."}'
cargo run -- source add --name feishu --config-json '{"kind":"feishu","appId":"cli_xxx","appSecret":"secret","folderToken":"..."}'
```

Use a JSON config file when you need non-default paths:

```json
{
  "databasePath": "./hoarder.db",
  "vaultPath": "./vault",
  "listenAddr": "127.0.0.1:4761",
  "jobConcurrency": 1,
  "fileConcurrency": 4,
  "logLevel": "info"
}
```

Pass it with `--config`:

```bash
cargo run -- --config ./hoarder.config.json serve
```
