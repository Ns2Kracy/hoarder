# Hoarder Development

Hoarder is a Rust binary with an Axum API, SQLite persistence, OpenDAL source connectors, a one-way sync engine, and a Svelte/Vite management console.

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

Use a JSON config file when you need non-default paths:

```json
{
  "databasePath": "./hoarder.db",
  "vaultPath": "./vault",
  "listenAddr": "127.0.0.1:4761",
  "jobConcurrency": 1,
  "fileConcurrency": 4
}
```

Pass it with `--config`:

```bash
cargo run -- --config ./hoarder.config.json serve
```
