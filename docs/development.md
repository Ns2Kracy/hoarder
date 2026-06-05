# Hoarder Development

Hoarder is a Rust workspace with a CLI binary, Axum API, SQLite persistence, OpenDAL source connectors, a multi-source one-way sync engine, and a Svelte/Vite management console.

## Product And Architecture Docs

- [Product PRD](prd.md): product positioning, MVP scope, user journeys, success metrics, and roadmap.
- [Architecture](architecture.md): current technical architecture, module boundaries, data model, API surface, and extension points.
- [Flows](flows.md): product and technical flows for source setup, job runs, source-to-vault decisions, vault writes, scheduler, errors, and settings.

## Prerequisites

- mise
- SQLite support through SeaORM/sqlx

## Build Order

The release binary embeds the built frontend from `web/dist`. Build the frontend before compiling Rust when you want the packaged UI to match the latest web source:

```bash
mise run setup
mise run release:build
```

During Rust compilation, `crates/hoarder-server/src/assets.rs` embeds the current contents of `web/dist` into the binary. Rebuild the frontend after changing files under `web/`.

## Verification

Run the full local quality gate before merging:

```bash
mise run verify
```

Run focused backend checks:

```bash
mise run rust:fmt
mise run rust:clippy
mise run rust:test
```

Run the frontend checks and build:

```bash
mise run web:verify
```

Run the release packaging check:

```bash
mise run release:build
```

## Local Run

Start the packaged app on the default loopback address:

```bash
mise run serve
```

Open `http://127.0.0.1:4761`. API routes are available under `/api/*`; frontend routes are served from the embedded app shell.

Override the server address:

```bash
mise run serve -- --addr 127.0.0.1:4762
```

Sync the SQLite schema for the configured database:

```bash
mise run db:sync
```

Create and run a local filesystem workflow from the CLI:

```bash
mise run cli -- source add --name docs --service fs --root ./docs
mise run cli -- source list
mise run cli -- source test --id 1
mise run cli -- job add --source-id 1 --name docs --interval 300
mise run cli -- job list
mise run cli -- sync run --job-id 1
mise run cli -- sync status
```

Create app connector sources through the generic JSON config path:

```bash
mise run cli -- source add --name notion --config-json '{"kind":"notion","token":"secret","dataSourceId":"..."}'
mise run cli -- source add --name feishu --config-json '{"kind":"feishu","appId":"cli_xxx","appSecret":"secret","folderToken":"..."}'
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
mise run cli -- --config ./hoarder.config.json serve
```
