# Hoarder

[中文文档](README.zh-CN.md)

Hoarder is a local-first data aggregation and one-way sync platform. It also acts as a local knowledge-base aggregation layer for AI and RAG workflows: it connects external sources, writes their content into a readable local vault, and records sync state in SQLite so runs can be inspected, retried, and audited from a local CLI, API, or web console.

The first implementation focuses on a strong local foundation: Rust, Axum, SeaORM 2.0 entity-first, SQLite, OpenDAL, Svelte, Tailwind CSS, Bun, and a release path that embeds the frontend into one Rust binary.

## Highlights

- Local-first by default: data is written to your own filesystem and metadata is stored in local SQLite.
- One-way sync model: sources write into the vault; local files are not pushed back to sources.
- Readable vault layout: synced files live under `vault/{source_id}/normalized/source/path`.
- Connector abstraction: sync logic depends on Hoarder traits, not OpenDAL or vendor-specific APIs.
- Built-in connector families: OpenDAL-backed `fs`, `webdav`, `sftp`, and `s3`, plus Notion and Feishu virtual-document connectors for knowledge-base aggregation.
- Safe writes: files stream through a temporary path and are atomically promoted into the vault.
- No automatic local deletion: missing source files are marked `deleted_on_source`, but local vault files remain.
- Structured run history: runs, items, errors, counts, hashes, and timestamps are persisted.
- RAG-ready data foundation: source paths, local paths, hashes, run history, and errors are preserved so future parsing, chunking, indexing, and retrieval can keep provenance.
- Structured request logging: Axum middleware records method, path, version, status, latency, user agent, and request id.
- Single binary packaging: the Rust release binary embeds `web/dist` assets.
- Strict quality gate: Rust warnings and strict Clippy groups are denied in `Cargo.toml`.

## Quick Start

Prerequisites:

- Rust 2024 toolchain
- Bun

Build the web UI, then run the local server:

```bash
cd web
bun install
bun run build
cd ..
cargo run -- serve
```

Open:

```text
http://127.0.0.1:4761
```

Use a custom config:

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

```bash
cargo run -- --config ./hoarder.config.json serve
```

## Project Docs

- [Product PRD](docs/prd.md): product positioning, MVP scope, user journeys, success metrics, and roadmap.
- [Architecture](docs/architecture.md): current technical architecture, module boundaries, data model, API surface, and extension points.
- [Flows](docs/flows.md): product and technical flows for source setup, job runs, sync decisions, vault writes, scheduler, errors, and settings.

## Commands

| Command | Status | Description |
| --- | --- | --- |
| `cargo run -- serve` | [x] | Start the Axum API and embedded web console. |
| `cargo run -- serve --addr 127.0.0.1:4762` | [x] | Override the listen address. |
| `cargo run -- --config ./hoarder.config.json serve` | [x] | Load JSON config before serving. |
| `cargo run -- db sync` | [x] | Synchronize the SQLite schema from SeaORM entities. |
| `cargo run -- source list` | [x] | List configured sources from SQLite. |
| `cargo run -- source add --name docs --service fs --root ./docs` | [x] | Create an OpenDAL filesystem source. |
| `cargo run -- source add --name notion --config-json '{"kind":"notion","token":"secret","dataSourceId":"..."}'` | [x] | Create a Notion virtual-document source. |
| `cargo run -- source add --name feishu --config-json '{"kind":"feishu","appId":"cli_xxx","appSecret":"secret","folderToken":"..."}'` | [x] | Create a Feishu Drive virtual-document source. |
| `cargo run -- source test --id 1` | [x] | Validate a source and persist health. |
| `cargo run -- job add --source-id 1 --name docs --interval 300` | [x] | Create a manual or interval sync job. |
| `cargo run -- job list` | [x] | List configured sync jobs. |
| `cargo run -- sync run --job-id 1` | [x] | Run one sync job immediately. |
| `cargo run -- sync status` | [x] | Print sync run status summaries. |

## Feature Checklist

### Core Platform

- [x] Rust 2024 backend
- [x] Tokio async runtime
- [x] Axum local HTTP API
- [x] Local-first default bind address, `127.0.0.1:4761`
- [x] JSON configuration file support
- [x] Clap-based CLI parser
- [x] Local integer identifiers
- [x] Standard-library filesystem paths
- [x] Strict Rust and Clippy lints in `Cargo.toml`
- [x] Release profile with LTO and symbol stripping
- [x] Background scheduler inside serve mode
- [x] Runtime settings persistence, mutation, and live application
- [ ] Multi-user remote deployment mode
- [ ] Authentication and authorization

### Persistence

- [x] SQLite metadata database
- [x] SeaORM 2.0 entity-first model definitions
- [x] Entity registry schema sync
- [x] `source` records
- [x] `sync_job` records
- [x] `sync_run` records
- [x] `sync_item` records
- [x] `sync_error` records
- [x] Repository abstraction for sync engine tests
- [x] SeaORM repository implementation
- [x] Durable app settings beyond the config file
- [ ] Database pruning or retention policies

### Connectors

- [x] Connector trait boundary
- [x] Connector capability model
- [x] `opendal` connector kind
- [x] `notion` and `feishu` connector kinds
- [x] OpenDAL service config validation for `fs`
- [x] OpenDAL service config validation for `webdav`
- [x] OpenDAL service config validation for `sftp`
- [x] OpenDAL service config validation for `s3`
- [x] Secret redaction for connector options
- [x] OpenDAL filesystem scan
- [x] OpenDAL filesystem file read
- [x] Directory and file metadata mapping into Hoarder snapshots
- [x] OpenDAL WebDAV operator implementation
- [x] OpenDAL SFTP operator implementation
- [x] OpenDAL S3 operator implementation
- [x] NAS-specific presets or templates
- [x] Notion connector implementation
- [x] Feishu connector implementation
- [x] Connector pagination or incremental cursor support
- [x] Pluggable third-party connector ABI contract

### Sync Runtime

- [x] One-way sync from source to local vault
- [x] Source path normalization
- [x] Absolute path rejection
- [x] Path traversal rejection
- [x] Reserved `.hoarder` path protection
- [x] Readable vault layout under `vault/{source_id}/...`
- [x] Temporary file writes
- [x] Atomic promotion into final vault path
- [x] SHA-256 content hash recording
- [x] New item detection
- [x] Changed item detection by item type, ETag, size, modified time, and hash
- [x] Unchanged item skipping
- [x] Source deletion detection
- [x] Missing source items marked `deleted_on_source`
- [x] Local vault files retained when source items disappear
- [x] Per-item failures recorded without failing the entire run
- [x] Connector-level failures fail the run
- [x] Run summaries with processed, synced, skipped, failed, and byte counts
- [x] Bounded concurrent file sync execution
- [x] Job-level concurrency control
- [x] Scheduled recurring sync jobs
- [x] Resume from connector cursor
- [ ] Retry policy for transient connector errors
- [ ] Conflict resolution
- [ ] Bidirectional sync
- [ ] Automatic local deletion policy

### API

- [x] `GET /api/health`
- [x] `GET /api/openapi.json`
- [x] `GET /api/sources`
- [x] `POST /api/sources`
- [x] `GET /api/jobs`
- [x] `POST /api/jobs/{id}/run`
- [x] `GET /api/runs`
- [x] `GET /api/items`
- [x] `GET /api/errors`
- [x] `GET /api/settings`
- [x] Stable structured error response shape
- [x] Internal database and IO details hidden from API errors
- [x] API fallback keeps unknown `/api/*` routes JSON-shaped
- [x] `POST /api/sources/{id}/test`
- [x] Structured request logging middleware
- [x] `POST /api/jobs`
- [x] `GET /api/runs/{id}`
- [x] Filtered item listing by source or status
- [x] `PATCH /api/settings`
- [x] OpenAPI specification

### Web Console

- [x] Svelte 5 frontend
- [x] Vite 8 build
- [x] Tailwind CSS 4 styling through `@tailwindcss/vite`
- [x] Bun-based frontend install, check, and build scripts
- [x] Embedded production assets served by Axum
- [x] Responsive sidebar layout
- [x] Overview page
- [x] Sources page
- [x] Source creation form for OpenDAL-style config
- [x] Jobs page
- [x] Runs page
- [x] Settings page
- [x] Status badges and compact operational tables
- [x] API client with mock fallback while the local API is unavailable
- [x] Full live wiring for the MVP workflow controls
- [x] Source test action backed by API route
- [x] Settings save backed by API route
- [x] Run detail endpoint integration
- [ ] Accessibility pass with keyboard and screen-reader checks
- [ ] Browser screenshot regression checks

### Packaging And Quality

- [x] Single Rust binary embeds frontend assets from `web/dist`
- [x] `cargo fmt --check`
- [x] Strict `cargo clippy --all-targets --all-features`
- [x] `cargo test`
- [x] `bun run verify`
- [x] `cargo build --release`
- [x] End-to-end local filesystem sync test
- [x] Static asset fallback tests
- [x] App service integration tests
- [x] API route tests
- [x] CLI command integration tests
- [x] Connector contract tests
- [x] Vault writer safety tests
- [ ] CI workflow
- [ ] Release artifacts for macOS, Linux, and Windows
- [ ] Installer or package manager distribution
- [ ] Performance benchmarks
- [ ] Long-running soak tests

### Product Roadmap

- [ ] Full-text search
- [ ] RAG-ready parsing, chunking, indexing, and retrieval API
- [ ] Bidirectional sync
- [ ] Automatic local deletion policy
- [ ] Cross-source deduplication
- [ ] Tagging or collections
- [ ] Notifications
- [ ] Import/export of source definitions

## Architecture

```text
CLI / Web UI
  -> command handlers / Axum API
  -> sync engine
  -> connector trait
  -> source connector
  -> vault writer
  -> SeaORM repository
  -> SQLite
```

Key boundaries:

- `src/core`: stable domain types shared across layers.
- `src/connectors`: connector traits and OpenDAL-backed implementation.
- `src/sync`: planner, engine, repository trait, and vault writer.
- `src/db`: SeaORM repository and schema sync.
- `src/api`: DTOs, routes, state traits, and error mapping.
- `src/server.rs`: Axum server assembly and database-backed API wiring.
- `web`: Svelte management console.

## Local Vault Layout

```text
vault/
  {source_id}/
    normalized/source/path.ext
  .hoarder/
    tmp/
```

Source paths are normalized before writing. Hoarder rejects absolute paths, traversal paths, NUL bytes, Windows drive prefixes, and attempts to write into the reserved `.hoarder` directory.

## Development

Install frontend dependencies:

```bash
cd web
bun install
```

Run frontend verification:

```bash
cd web
bun run verify
```

Run backend verification:

```bash
cargo fmt --check
cargo clippy --all-targets --all-features --message-format=short
cargo test
```

Build the packaged release binary:

```bash
cd web
bun run build
cd ..
cargo build --release
```

## Current Status

Hoarder is an early local-first MVP. The backend serves the embedded web console, syncs SQLite schema, exposes the MVP control plane API, executes source/job/sync CLI workflows, runs fixed-interval jobs in serve mode, and passes end-to-end filesystem sync tests. OpenDAL operator wiring now covers filesystem, WebDAV, SFTP, and S3; the next highest-value work is connector-specific integration tests, typed config templates, and CI/release artifacts.
