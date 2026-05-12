# Hoarder Connector Platform Design

Date: 2026-05-12

## Objective

Hoarder is a local-first data aggregation and synchronization tool. It connects multiple external data sources, synchronizes them one-way into a local vault, and exposes a local CLI plus Web UI for configuration, status, and troubleshooting.

The first release focuses on a connector platform rather than a single OpenDAL-only tool. OpenDAL is the first storage connector backend, while the internal connector contract leaves room for application connectors such as Notion and Feishu.

## Confirmed Scope

- Multiple data sources synchronize into one local vault.
- Synchronization is one-way: `source -> local vault`.
- The first source connector family is OpenDAL-backed storage.
- Local files use a hybrid layout: readable paths plus SQLite metadata.
- No full-text search in the first release.
- No bidirectional sync in the first release.
- No automatic local deletion in the first release.
- CLI uses `clap`.
- Backend uses Rust, Axum, SQLite, and SeaORM 2.0 entity-first.
- Frontend uses Tailwind CSS and should be designed before implementation.
- Release target is a single binary with embedded frontend assets.

## Architecture

```text
CLI / Web UI
  -> command handlers / Axum API
  -> sync engine
  -> connector registry
  -> source connector
  -> local vault writer
  -> SeaORM repository
  -> SQLite
```

The system has four major layers:

1. Connector layer
   - Defines Hoarder's connector contracts.
   - Provides `SourceConnector` implementations.
   - First implementation: `OpenDalSourceConnector`.
   - Future implementations: Notion, Feishu, and other application APIs.

2. Sync engine layer
   - Streams source snapshots.
   - Compares snapshots against stored item state.
   - Writes changed files into the local vault.
   - Records run, item, and error state.

3. Repository layer
   - Uses SQLite through SeaORM 2.0 entity-first.
   - Entities are the source of truth for schema shape.
   - Schema sync runs at startup for local development and first release ergonomics.

4. Interface layer
   - CLI for serve, source management, manual sync, status, and database sync.
   - Axum API for the local Web UI.
   - Frontend management console built with Tailwind CSS.

## Connector Contract

Connectors expose Hoarder domain semantics instead of leaking OpenDAL directly.

```text
SourceConnector
  - kind() -> ConnectorKind
  - validate(config) -> Result<ConnectorCapabilities>
  - scan(cursor) -> stream<ItemSnapshot>
  - read(item_ref) -> byte stream
```

`ItemSnapshot` is the sync engine's minimum common model:

```text
source_id
source_path
item_type: file | directory | virtual_document
size
etag?
modified_at?
content_hash?
metadata_json?
```

OpenDAL connectors map object and filesystem operations into this model. Future application connectors can export pages or documents as `virtual_document` items and write markdown, JSON, HTML, and attachments into the vault.

## Local Vault Layout

The vault keeps files easy to inspect while SQLite tracks identity and state.

```text
vault/
  {source_id}/
    normalized/source/path.ext
  .hoarder/
    tmp/
```

Rules:

- Preserve readable source-relative paths under `vault/{source_id}/`.
- Normalize every source path before writing.
- Reject absolute paths.
- Reject `..` traversal.
- Prevent source items from overwriting `.hoarder/`.
- Write to a temporary file first, then atomically rename into place.
- Mark source deletions as `deleted_on_source`; do not delete local files by default.

## Database Model

SeaORM 2.0 entity-first is used. Entities live under `src/entity/`.

Core entities:

```text
app_setting
source
sync_job
sync_run
sync_item
sync_error
```

`sync_item` tracks item identity and change state:

```text
source_id
source_path
target_path
item_type
size
etag
modified_at
content_hash
sync_status: pending | synced | failed | skipped | deleted_on_source
last_synced_at
last_error
```

SeaORM's entity-first workflow has been available since SeaORM 2.0.0. It lets developers hand-write entity files and use schema registry sync to create or update tables from those entities. It requires the `schema-sync` and `entity-registry` features.

Source: https://www.sea-ql.org/SeaORM/docs/generate-entity/entity-first/

## CLI

The CLI uses `clap` and exposes these initial commands:

```text
hoarder serve --config ./hoarder.toml --addr 127.0.0.1:4761
hoarder db sync
hoarder source list
hoarder source add opendal --kind fs --name local --root ./sample
hoarder source test <source-id>
hoarder sync run [job-id]
hoarder sync status
```

`serve` starts Axum and the scheduler. `sync run` performs a one-shot sync without serving the Web UI.

## Axum API

The API is local-first and listens on `127.0.0.1` by default.

```text
GET    /api/health
GET    /api/sources
POST   /api/sources
POST   /api/sources/:id/test
GET    /api/jobs
POST   /api/jobs
POST   /api/jobs/:id/run
GET    /api/runs
GET    /api/runs/:id
GET    /api/items?sourceId=&status=
GET    /api/errors
GET    /api/settings
PATCH  /api/settings
```

All API errors use a stable response shape:

```json
{
  "error": {
    "code": "VALIDATION_ERROR",
    "message": "Invalid source configuration",
    "details": {}
  }
}
```

## Frontend Management Console

The frontend is a practical local admin console, not a landing page.

Pages:

```text
Overview
Sources
Jobs
Runs
Settings
```

Design constraints:

- Tailwind CSS.
- Dense, tool-oriented layout.
- Sidebar navigation.
- Tables and status badges for operational state.
- Forms in dialogs or panels.
- Logs and error details in monospaced blocks.
- No decorative hero sections.
- No search UI in the first release.

Recommended stack: Svelte + Vite + Tailwind. The interface is configuration and status heavy, so Svelte keeps component code and runtime weight low. React or Solid remain acceptable if implementation constraints change.

## Sync Runtime

Run modes:

```text
manual run       CLI or Web UI starts one run
scheduled run    job interval triggers inside serve mode
serve mode       Axum plus scheduler
cli mode         one command, then exit
```

Concurrency defaults:

```text
job concurrency   1
file concurrency  4
```

Change detection order:

```text
1. Check source_path in sync_items.
2. Compare size, etag, and modified_at.
3. Compute content_hash only when needed.
4. Sync changed or new items.
```

The engine should stream directory scans and file reads. Large directories and files must not be loaded fully into memory.

OpenDAL's Rust API centers on `Operator`, which provides a unified interface for services such as filesystem, WebDAV, S3, FTP, SFTP, Google Drive, OneDrive, Dropbox, Aliyun Drive, and SQLite-backed service adapters.

Sources:

- https://opendal.apache.org/docs/rust/opendal/
- https://opendal.apache.org/docs/rust/opendal/services/index.html
- https://opendal.apache.org/docs/rust/opendal/struct.Operator.html

## Error Handling

- A single item failure does not fail the whole run.
- Connector-level failures fail the run.
- Every run records start time, end time, processed count, success count, failed count, skipped count, and byte count.
- Item failures write rows to `sync_error`.
- API responses return structured errors without internal stack traces.
- Secrets are never logged.

## Security Defaults

- Web UI binds to `127.0.0.1` by default.
- Secrets are redacted in logs and API responses.
- All external input is validated at the boundary.
- SQLite queries go through SeaORM.
- Source paths are normalized and constrained to the vault root.
- Temporary writes are atomically promoted.
- Local deletion is disabled by default.

## Performance Targets

Initial targets:

```text
idle memory        < 80 MB
startup            < 1 second for normal local startup
file sync          streaming read/write
directory scan     bounded memory growth
release artifact   single binary
```

## Out of Scope For First Release

- Full-text search.
- Bidirectional sync.
- Conflict resolution.
- Automatic local deletion.
- Notion connector implementation.
- Feishu connector implementation.
- Remote multi-user auth.
- Plugin ABI for third-party compiled plugins.
