# Hoarder Connector Platform Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build a local-first connector platform that syncs multiple OpenDAL-backed sources one-way into a local vault, with CLI, Axum API, SeaORM 2.0 entity-first persistence, and a Tailwind management UI.

**Architecture:** Define shared domain contracts first, then split work across independent backend, connector, sync, CLI/API, and frontend lanes. OpenDAL is isolated behind a Hoarder connector trait. SQLite schema is defined by SeaORM entities.

**Tech Stack:** Rust 2024, Axum, Tokio, Clap, SeaORM 2.0 entity-first, SQLite, OpenDAL, Svelte + Vite + Tailwind CSS, Bun.

---

## Coordination Rules For Multiple Agents

- Start with Phase 1 only. Do not parallelize before the shared contracts compile.
- Each agent owns only the files listed in its lane.
- Agents must not reformat or rewrite files owned by another lane.
- Shared contract changes require coordination through `src/core/` and must happen before dependent tasks.
- Commit after each task that builds and passes its focused tests.
- Run `cargo test` before merging backend branches.
- Use Bun for all frontend dependency and script commands.
- Run frontend build/check commands before merging frontend branches.

## Suggested Worktrees

Create one branch/worktree per lane after Phase 1 is complete:

```bash
git worktree add ../hoarder-db feature/db-entities
git worktree add ../hoarder-connectors feature/opendal-connectors
git worktree add ../hoarder-sync feature/sync-engine
git worktree add ../hoarder-api feature/api-cli
git worktree add ../hoarder-web feature/web-ui
```

Merge order:

```text
1. foundation
2. db-entities
3. opendal-connectors
4. sync-engine
5. api-cli
6. web-ui
7. packaging-integration
```

## Copy-Paste Execution Prompts

Use this order:

1. Run the foundation prompt in the main worktree.
2. Commit foundation.
3. Create the worktrees above.
4. Run the DB, Connector, Sync, CLI/API, and Web prompts in parallel.
5. Merge those branches after their verification passes.
6. Run the Integration prompt last.

### Foundation Prompt

```text
Implement Phase 1, Tasks 1-4 from docs/plans/2026-05-12-hoarder-connector-platform-implementation.md.

You own the shared foundation only:
- Cargo.toml
- src/main.rs
- src/lib.rs
- src/config.rs
- src/error.rs
- src/core/**
- src/connectors/mod.rs
- src/connectors/traits.rs
- src/connectors/registry.rs
- tests/core_types.rs
- tests/vault_path.rs
- tests/connector_contract.rs

Do not implement DB entities, OpenDAL connector internals, sync engine, Axum routes, CLI commands beyond a thin entrypoint, or frontend files.

Follow the plan task-by-task. Run:
- cargo build
- cargo test

Commit only the foundation changes when verification passes. Return changed files, test results, and any contract decisions future agents must follow.
```

### Parallel DB Prompt

```text
Implement Tasks 5-6 from docs/plans/2026-05-12-hoarder-connector-platform-implementation.md.

Ownership:
- src/entity/**
- src/db/**
- tests/db_schema.rs

Use SeaORM 2.0 entity-first. Keep entities as the schema source of truth. Do not edit connector, sync, API, CLI, or frontend files unless compilation is blocked by a shared contract; if blocked, report the needed contract change instead of making broad edits.

Run:
- cargo check
- cargo test db_schema

Commit only DB-lane changes when verification passes. Return changed files, test results, and any assumptions about repository traits.
```

### Parallel Connector Prompt

```text
Implement Tasks 7-8 from docs/plans/2026-05-12-hoarder-connector-platform-implementation.md.

Ownership:
- src/connectors/opendal/**
- connector-related updates to src/connectors/mod.rs only if needed
- tests/opendal_config.rs
- tests/opendal_fs_connector.rs

Implement OpenDAL config validation/redaction and the filesystem source connector first. Do not expose OpenDAL types through SourceConnector. Do not edit sync engine, API, DB repository, or frontend files.

Run:
- cargo test opendal_config
- cargo test opendal_fs_connector

Commit only connector-lane changes when verification passes. Return changed files, test results, and supported OpenDAL service kinds.
```

### Parallel Sync Prompt

```text
Implement Tasks 9-11 from docs/plans/2026-05-12-hoarder-connector-platform-implementation.md.

Ownership:
- src/sync/**
- tests/vault_writer.rs
- tests/sync_planner.rs
- tests/sync_engine.rs

Depend on connector and repository traits. Do not import concrete OpenDAL types or SeaORM entities in the sync engine. Preserve the one-way source-to-vault behavior and default no-local-delete policy.

Run:
- cargo test vault_writer
- cargo test sync_planner
- cargo test sync_engine

Commit only sync-lane changes when verification passes. Return changed files, test results, and how failed item handling works.
```

### Parallel CLI/API Prompt

```text
Implement Tasks 12-15 from docs/plans/2026-05-12-hoarder-connector-platform-implementation.md.

Ownership:
- src/cli.rs
- src/api/**
- src/server.rs
- src/main.rs
- tests/cli_parse.rs
- tests/api_error.rs
- tests/api_routes.rs

Keep route handlers thin. Use service and repository abstractions. Default server binding must remain 127.0.0.1. Do not implement frontend or rewrite sync/DB/connector internals.

Run:
- cargo test cli_parse
- cargo test api_error
- cargo test api_routes
- cargo run -- serve --addr 127.0.0.1:4761

Stop the server after confirming startup. Commit only CLI/API-lane changes when verification passes. Return changed files, endpoint list, and test results.
```

### Parallel Web Prompt

```text
Implement Tasks 16-19 from docs/plans/2026-05-12-hoarder-connector-platform-implementation.md.

Ownership:
- web/**

Use Svelte + Vite + Tailwind CSS + Bun. Build a real local management console, not a landing page. Use mocked data until API endpoints are available, then route through web/src/lib/api.ts. Keep UI compact and operational: Overview, Sources, Jobs, Runs, Settings.

Run:
- cd web
- bun install
- bun run build

Commit only web-lane changes when verification passes. Return changed files, build summary, and screenshots if browser verification is available.
```

### Final Integration Prompt

```text
Implement Tasks 20-22 from docs/plans/2026-05-12-hoarder-connector-platform-implementation.md after DB, Connector, Sync, CLI/API, and Web branches are merged.

Ownership:
- src/assets.rs
- static serving integration in src/server.rs
- tests/e2e_local_fs_sync.rs
- docs/development.md
- integration-only fixes needed to make the full system work

Do not redesign frontend or rewrite sync internals. Use Bun for frontend commands.

Run:
- cargo test
- cd web
- bun run build
- cd ..
- cargo build --release

Commit only integration changes when verification passes. Return full verification results and remaining release risks.
```

## Phase 1: Shared Foundation

This phase is sequential. Finish and commit it before parallel implementation.

### Task 1: Project Dependencies And Module Skeleton

**Owner:** Foundation agent

**Files:**

- Modify: `Cargo.toml`
- Modify: `src/main.rs`
- Create: `src/lib.rs`
- Create: `src/core/mod.rs`
- Create: `src/config.rs`
- Create: `src/error.rs`

**Steps:**

1. Add dependencies for Tokio, Axum, Clap, Serde, thiserror, tracing, SeaORM 2.0, SQLite support, OpenDAL, futures, bytes, chrono, uuid, and camino.
2. Convert `main.rs` into a thin async entrypoint.
3. Add `lib.rs` with public module declarations.
4. Add `AppError` and `AppResult<T>` in `src/error.rs`.
5. Add `AppConfig` with defaults for database path, vault path, listen address, and concurrency.

**Acceptance criteria:**

- `cargo build` succeeds.
- `cargo test` succeeds.
- `src/main.rs` contains no business logic.

**Verification:**

```bash
cargo build
cargo test
```

**Dependencies:** None

**Estimated scope:** Medium

### Task 2: Core Domain Types

**Owner:** Foundation agent

**Files:**

- Create: `src/core/types.rs`
- Modify: `src/core/mod.rs`
- Create: `tests/core_types.rs`

**Steps:**

1. Define `SourceId`, `JobId`, `RunId`, and `ItemId` newtypes.
2. Define `ItemType`, `SyncStatus`, `ConnectorKind`, and `ConnectorCapabilities`.
3. Define `ItemSnapshot` and `ItemRef`.
4. Add serde derives for API usage.
5. Add focused tests for enum serialization and basic snapshot construction.

**Acceptance criteria:**

- Types compile and are serializable.
- Tests cover at least one `file`, `directory`, and `virtual_document` item type.

**Verification:**

```bash
cargo test core_types
```

**Dependencies:** Task 1

**Estimated scope:** Small

### Task 3: Vault Path Normalization

**Owner:** Foundation agent

**Files:**

- Create: `src/core/vault_path.rs`
- Modify: `src/core/mod.rs`
- Create: `tests/vault_path.rs`

**Steps:**

1. Implement `normalize_source_path(input: &str) -> AppResult<String>`.
2. Reject empty paths, absolute paths, `..`, Windows drive prefixes, and `.hoarder` root paths.
3. Implement `target_path(vault_root, source_id, normalized_path)`.
4. Test valid nested paths and invalid traversal cases.

**Acceptance criteria:**

- Path traversal cannot escape the vault.
- `.hoarder` cannot be overwritten by source items.
- Windows-style absolute paths are rejected even on Unix.

**Verification:**

```bash
cargo test vault_path
```

**Dependencies:** Task 1

**Estimated scope:** Small

### Task 4: Connector Trait Contracts

**Owner:** Foundation agent

**Files:**

- Create: `src/connectors/mod.rs`
- Create: `src/connectors/traits.rs`
- Create: `src/connectors/registry.rs`
- Create: `tests/connector_contract.rs`

**Steps:**

1. Define `SourceConnector` with `kind`, `validate`, `scan`, and `read`.
2. Return boxed async streams for scans and byte streams for reads.
3. Define `ConnectorConfig` as a typed enum with an `OpenDal` variant placeholder.
4. Implement a minimal registry that can register and retrieve connector factories by kind.
5. Add tests using a fake connector.

**Acceptance criteria:**

- The sync engine can depend on traits without importing OpenDAL.
- Registry tests prove lookup succeeds for a registered connector and fails for missing kinds.

**Verification:**

```bash
cargo test connector_contract
```

**Dependencies:** Tasks 1-2

**Estimated scope:** Medium

## Checkpoint: Foundation

Before parallel work starts:

- `cargo test` passes.
- `cargo build` passes.
- Shared types and connector contracts are committed.
- No frontend files exist unless explicitly added by frontend lane.

## Phase 2: Parallel Backend Lanes

These lanes can start after the foundation checkpoint.

### Task 5: SeaORM Entity Modules

**Owner:** DB agent

**Files:**

- Create: `src/entity/mod.rs`
- Create: `src/entity/prelude.rs`
- Create: `src/entity/app_setting.rs`
- Create: `src/entity/source.rs`
- Create: `src/entity/sync_job.rs`
- Create: `src/entity/sync_run.rs`
- Create: `src/entity/sync_item.rs`
- Create: `src/entity/sync_error.rs`

**Steps:**

1. Add SeaORM 2.0 dense/entity-first entity definitions.
2. Define relations from source to jobs, runs, items, and errors.
3. Use string columns for external IDs and enum-like status values unless SeaORM enum mapping is already simple.
4. Ensure entities register for schema sync.

**Acceptance criteria:**

- Entities compile under SeaORM 2.0.
- Entity modules do not contain repository business logic.

**Verification:**

```bash
cargo check
```

**Dependencies:** Tasks 1-2

**Estimated scope:** Medium

### Task 6: Database Connection And Schema Sync

**Owner:** DB agent

**Files:**

- Create: `src/db/mod.rs`
- Create: `src/db/schema.rs`
- Create: `src/db/repository.rs`
- Create: `tests/db_schema.rs`

**Steps:**

1. Implement SQLite connection setup.
2. Implement `sync_schema(db)` using SeaORM entity registry.
3. Implement repository methods for creating/listing sources and jobs.
4. Add an in-memory or temp-file SQLite test that syncs schema and inserts a source.

**Acceptance criteria:**

- `sync_schema` creates all required tables.
- Source CRUD works through SeaORM.
- Test uses an isolated database.

**Verification:**

```bash
cargo test db_schema
```

**Dependencies:** Task 5

**Estimated scope:** Medium

### Task 7: OpenDAL Connector Config

**Owner:** Connector agent

**Files:**

- Create: `src/connectors/opendal/mod.rs`
- Create: `src/connectors/opendal/config.rs`
- Modify: `src/connectors/mod.rs`
- Modify: `src/connectors/traits.rs`
- Create: `tests/opendal_config.rs`

**Steps:**

1. Define OpenDAL service config variants for `fs`, `webdav`, `sftp`, and `s3`.
2. Add serde support for storing config JSON.
3. Add secret-redaction helpers for API/log output.
4. Validate required fields for each service kind without opening a network connection.

**Acceptance criteria:**

- Config validation catches missing required fields.
- Redacted config never exposes password, token, access key, or secret key values.

**Verification:**

```bash
cargo test opendal_config
```

**Dependencies:** Task 4

**Estimated scope:** Medium

### Task 8: OpenDAL Source Connector

**Owner:** Connector agent

**Files:**

- Create: `src/connectors/opendal/source.rs`
- Modify: `src/connectors/opendal/mod.rs`
- Create: `tests/opendal_fs_connector.rs`

**Steps:**

1. Build an OpenDAL `Operator` from validated config.
2. Implement `scan` for filesystem service first.
3. Implement `read` as a byte stream.
4. Map OpenDAL metadata into `ItemSnapshot`.
5. Add tests using a temporary local directory and the `fs` service.

**Acceptance criteria:**

- The connector lists nested files from a temp directory.
- The connector reads file contents as a stream.
- The connector does not expose OpenDAL types through the trait boundary.

**Verification:**

```bash
cargo test opendal_fs_connector
```

**Dependencies:** Task 7

**Estimated scope:** Medium

### Task 9: Vault Writer

**Owner:** Sync agent

**Files:**

- Create: `src/sync/mod.rs`
- Create: `src/sync/vault_writer.rs`
- Create: `tests/vault_writer.rs`

**Steps:**

1. Implement temp-file write under `vault/.hoarder/tmp`.
2. Stream bytes into the temp file.
3. Compute content hash while writing.
4. Atomically rename to the final normalized target path.
5. Clean up temp file on failure.

**Acceptance criteria:**

- Successful writes create expected files.
- Failed writes do not leave final partial files.
- Hash is returned after write.

**Verification:**

```bash
cargo test vault_writer
```

**Dependencies:** Task 3

**Estimated scope:** Medium

### Task 10: Sync Planner

**Owner:** Sync agent

**Files:**

- Create: `src/sync/planner.rs`
- Create: `tests/sync_planner.rs`

**Steps:**

1. Compare `ItemSnapshot` with stored item state.
2. Return decisions: `Sync`, `Skip`, `MarkDeleted`.
3. Prefer etag, modified_at, and size before content hash.
4. Keep the planner pure and independent of database and filesystem.

**Acceptance criteria:**

- New items sync.
- Same size/etag/mtime items skip.
- Changed etag or size syncs.
- Missing source items can be marked deleted without deleting local files.

**Verification:**

```bash
cargo test sync_planner
```

**Dependencies:** Task 2

**Estimated scope:** Small

### Task 11: Sync Engine Run Loop

**Owner:** Sync agent

**Files:**

- Create: `src/sync/engine.rs`
- Modify: `src/sync/mod.rs`
- Create: `tests/sync_engine.rs`

**Steps:**

1. Implement `SyncEngine::run_job(job_id)`.
2. Stream snapshots from a `SourceConnector`.
3. Use the planner to choose sync or skip.
4. Use `VaultWriter` for changed files.
5. Persist run/item/error updates through repository traits.
6. Add tests using a fake connector and fake repository.

**Acceptance criteria:**

- One failed item records an error and the run continues.
- Run summary counts processed, synced, skipped, and failed items.
- The engine depends on connector and repository traits, not concrete OpenDAL or SeaORM types.

**Verification:**

```bash
cargo test sync_engine
```

**Dependencies:** Tasks 9-10 and repository trait from Task 6

**Estimated scope:** Medium

## Phase 3: Interface Lanes

These can run in parallel once repository and engine APIs are stable.

### Task 12: Clap CLI Structure

**Owner:** CLI/API agent

**Files:**

- Create: `src/cli.rs`
- Modify: `src/main.rs`
- Create: `tests/cli_parse.rs`

**Steps:**

1. Define `serve`, `db sync`, `source list`, `source add`, `source test`, `sync run`, and `sync status`.
2. Parse global `--config` and `--log-level`.
3. Add parse-only tests for representative commands.

**Acceptance criteria:**

- CLI parse tests pass.
- Command definitions do not execute business logic directly.

**Verification:**

```bash
cargo test cli_parse
```

**Dependencies:** Task 1

**Estimated scope:** Small

### Task 13: Axum API Types And Error Shape

**Owner:** CLI/API agent

**Files:**

- Create: `src/api/mod.rs`
- Create: `src/api/error.rs`
- Create: `src/api/types.rs`
- Create: `tests/api_error.rs`

**Steps:**

1. Define `ApiErrorBody`.
2. Map `AppError` into HTTP status plus JSON error response.
3. Define request/response DTOs for sources, jobs, runs, items, errors, and settings.
4. Add tests for error serialization.

**Acceptance criteria:**

- Error response shape is stable.
- DTOs do not expose secret values.

**Verification:**

```bash
cargo test api_error
```

**Dependencies:** Tasks 1-2 and Task 7 for redacted config shape

**Estimated scope:** Medium

### Task 14: Axum API Routes

**Owner:** CLI/API agent

**Files:**

- Create: `src/api/routes.rs`
- Create: `src/api/state.rs`
- Modify: `src/api/mod.rs`
- Create: `tests/api_routes.rs`

**Steps:**

1. Add routes for health, sources, jobs, runs, items, errors, and settings.
2. Wire routes to repository and sync service traits.
3. Add route tests with fake services.
4. Keep handlers thin.

**Acceptance criteria:**

- `GET /api/health` returns success.
- `GET /api/sources` returns a JSON list.
- `POST /api/jobs/:id/run` triggers the sync service abstraction.

**Verification:**

```bash
cargo test api_routes
```

**Dependencies:** Tasks 6, 11, and 13

**Estimated scope:** Medium

### Task 15: Serve Command

**Owner:** CLI/API agent

**Files:**

- Create: `src/server.rs`
- Modify: `src/main.rs`
- Modify: `src/cli.rs`

**Steps:**

1. Load config.
2. Connect SQLite.
3. Run schema sync.
4. Build connector registry.
5. Build Axum router.
6. Bind to configured address.

**Acceptance criteria:**

- `hoarder serve --addr 127.0.0.1:4761` starts the API.
- Server defaults to local loopback.

**Verification:**

```bash
cargo run -- serve --addr 127.0.0.1:4761
```

Stop the server after confirming it starts.

**Dependencies:** Tasks 6, 8, 11, 12, and 14

**Estimated scope:** Medium

## Phase 4: Frontend Lane

Frontend work can begin with mocked API data after Task 13 defines DTOs. Final integration waits for Task 14.

### Task 16: Web Project Scaffold

**Owner:** Web agent

**Files:**

- Create: `web/package.json`
- Create: `web/vite.config.ts`
- Create: `web/tsconfig.json`
- Create: `web/src/main.ts`
- Create: `web/src/App.svelte`
- Create: `web/src/app.css`
- Create: `web/index.html`

**Steps:**

1. Scaffold Svelte + Vite + Tailwind using Bun.
2. Add app shell with sidebar and top status bar.
3. Define design tokens through Tailwind utility usage rather than custom one-off CSS values.
4. Use realistic mocked state.

**Acceptance criteria:**

- `bun install` succeeds.
- `bun run build` succeeds.
- First screen is the app console, not a landing page.

**Verification:**

```bash
cd web
bun install
bun run build
```

**Dependencies:** None, but align labels with design doc.

**Estimated scope:** Medium

### Task 17: Frontend API Client And State

**Owner:** Web agent

**Files:**

- Create: `web/src/lib/api.ts`
- Create: `web/src/lib/types.ts`
- Create: `web/src/lib/state.ts`

**Steps:**

1. Mirror backend DTOs from `src/api/types.rs`.
2. Implement typed fetch helpers.
3. Normalize API errors into one frontend error shape.
4. Add loading and empty state helpers.

**Acceptance criteria:**

- Components do not call `fetch` directly.
- Secret fields are not displayed.

**Verification:**

```bash
cd web
bun run build
```

**Dependencies:** Task 13

**Estimated scope:** Small

### Task 18: Overview And Sources Pages

**Owner:** Web agent

**Files:**

- Create: `web/src/routes/Overview.svelte`
- Create: `web/src/routes/Sources.svelte`
- Create: `web/src/components/StatusBadge.svelte`
- Create: `web/src/components/SourceForm.svelte`
- Modify: `web/src/App.svelte`

**Steps:**

1. Build Overview with source count, active jobs, last run, failed items, and vault size.
2. Build Sources table.
3. Build add-source form for OpenDAL source configs.
4. Add test connection action placeholder wired to API client.

**Acceptance criteria:**

- Layout is responsive at desktop and narrow widths.
- Form does not expose secret values after save.
- Text does not overflow buttons or table cells.

**Verification:**

```bash
cd web
bun run build
```

**Dependencies:** Tasks 16-17

**Estimated scope:** Medium

### Task 19: Jobs, Runs, And Settings Pages

**Owner:** Web agent

**Files:**

- Create: `web/src/routes/Jobs.svelte`
- Create: `web/src/routes/Runs.svelte`
- Create: `web/src/routes/Settings.svelte`
- Create: `web/src/components/RunSummaryTable.svelte`
- Modify: `web/src/App.svelte`

**Steps:**

1. Build Jobs table with run-now action.
2. Build Runs table and run detail panel.
3. Build Settings form for vault path, database path, concurrency, and log level.
4. Show structured errors in a monospaced block.

**Acceptance criteria:**

- User can navigate all MVP pages.
- Running, synced, skipped, failed, and deleted statuses have distinct visual treatment beyond color alone.
- Settings form uses normal form controls, not decorative cards.

**Verification:**

```bash
cd web
bun run build
```

**Dependencies:** Tasks 16-17

**Estimated scope:** Medium

## Phase 5: Integration

### Task 20: Static Asset Embedding

**Owner:** Integration agent

**Files:**

- Create: `src/assets.rs`
- Modify: `src/server.rs`
- Modify: `Cargo.toml`

**Steps:**

1. Embed `web/dist` assets in the Rust binary.
2. Serve frontend routes from Axum.
3. Keep `/api/*` routed to API handlers.
4. Add a build note documenting frontend build order.

**Acceptance criteria:**

- `cargo build --release` includes frontend assets after `web` has been built.
- Browser refresh on frontend routes returns the app shell.

**Verification:**

```bash
cd web
bun run build
cd ..
cargo build --release
```

**Dependencies:** Tasks 15-19

**Estimated scope:** Small

### Task 21: End-To-End Local FS Sync

**Owner:** Integration agent

**Files:**

- Create: `tests/e2e_local_fs_sync.rs`
- Modify: existing files only if integration gaps are found

**Steps:**

1. Create a temp source directory with nested files.
2. Create a temp vault and SQLite database.
3. Register an OpenDAL fs source and sync job.
4. Run the sync engine.
5. Assert files exist under `vault/{source_id}/`.
6. Assert SeaORM rows record synced items and run summary.

**Acceptance criteria:**

- End-to-end local filesystem sync passes.
- Running the same job twice skips unchanged files.
- Local files are not deleted when a source file disappears; rows are marked `deleted_on_source`.

**Verification:**

```bash
cargo test e2e_local_fs_sync
```

**Dependencies:** Tasks 6, 8, 11, and 15

**Estimated scope:** Medium

### Task 22: Packaging And Release Checks

**Owner:** Integration agent

**Files:**

- Create: `docs/development.md`
- Modify: `Cargo.toml`
- Modify: `web/package.json`

**Steps:**

1. Document build, test, and run commands.
2. Add release build profile settings if needed.
3. Ensure final commands are clear for Rust and frontend.
4. Run full verification.

**Acceptance criteria:**

- A fresh agent can build and run the project using docs.
- Release binary builds.
- Full backend test suite passes.
- Frontend build passes.

**Verification:**

```bash
cargo test
cd web
bun run build
cd ..
cargo build --release
```

**Dependencies:** Task 20

**Estimated scope:** Small

## Checkpoints

### Checkpoint A: Foundation

- Tasks 1-4 complete.
- `cargo test` passes.
- Core contracts are stable enough for parallel lanes.

### Checkpoint B: Backend Core

- Tasks 5-11 complete.
- OpenDAL fs source can sync into a temp vault through tests.
- `cargo test` passes.

### Checkpoint C: Interfaces

- Tasks 12-15 complete.
- `hoarder serve` starts locally.
- API route tests pass.

### Checkpoint D: Web UI

- Tasks 16-19 complete.
- Frontend build passes.
- UI covers Overview, Sources, Jobs, Runs, and Settings.

### Checkpoint E: Release Candidate

- Tasks 20-22 complete.
- `cargo test` passes.
- `cargo build --release` passes.
- Frontend assets are embedded.

## Agent Prompts

Use these prompts after the foundation checkpoint.

### DB Agent Prompt

Implement Tasks 5-6 from `docs/plans/2026-05-12-hoarder-connector-platform-implementation.md`.

Ownership: `src/entity/**`, `src/db/**`, and database-focused tests only. Do not edit connector, sync, API, CLI, or frontend files unless a shared contract blocks compilation; if blocked, report the needed contract change instead of making broad edits.

Return: changed files, tests run, and any contract assumptions.

### Connector Agent Prompt

Implement Tasks 7-8 from `docs/plans/2026-05-12-hoarder-connector-platform-implementation.md`.

Ownership: `src/connectors/opendal/**` and OpenDAL connector tests. Do not expose OpenDAL types through `SourceConnector`. Do not edit sync engine, API, DB repository, or frontend files.

Return: changed files, tests run, and supported OpenDAL service kinds.

### Sync Agent Prompt

Implement Tasks 9-11 from `docs/plans/2026-05-12-hoarder-connector-platform-implementation.md`.

Ownership: `src/sync/**` and sync tests. Depend on connector and repository traits. Do not import concrete OpenDAL types or SeaORM entities in the sync engine.

Return: changed files, tests run, and how failed item handling works.

### CLI/API Agent Prompt

Implement Tasks 12-15 from `docs/plans/2026-05-12-hoarder-connector-platform-implementation.md`.

Ownership: `src/cli.rs`, `src/api/**`, `src/server.rs`, `src/main.rs`, and API/CLI tests. Keep route handlers thin and use service/repository abstractions.

Return: changed files, tests run, and endpoint list.

### Web Agent Prompt

Implement Tasks 16-19 from `docs/plans/2026-05-12-hoarder-connector-platform-implementation.md`.

Ownership: `web/**` only. Build a real management console with Tailwind CSS. Do not add a landing page. Use mocked data until API endpoints are available, then route through `web/src/lib/api.ts`. Use Bun for all frontend commands: `bun install`, `bun run build`, and any additional package scripts.

Return: changed files, build command output summary, and screenshots if browser verification is available.

### Integration Agent Prompt

Implement Tasks 20-22 only after backend and frontend lanes are merged.

Ownership: `src/assets.rs`, static serving integration, e2e tests, and development docs. Do not redesign frontend or rewrite sync internals; fix only integration gaps.

Return: changed files, full verification commands, and remaining release risks.

## Risks And Mitigations

| Risk | Impact | Mitigation |
| --- | --- | --- |
| Shared contracts change during parallel work | High | Freeze `src/core/**` and `src/connectors/traits.rs` after Phase 1; coordinate any changes explicitly. |
| SeaORM 2.0 entity-first API changes | Medium | Check official SeaORM 2.0 docs before implementation and keep entity sync isolated in `src/db/schema.rs`. |
| OpenDAL service differences | Medium | Test `fs` first; treat WebDAV/SFTP/S3 as config and connection-validation follow-ups. |
| Frontend and API DTO drift | Medium | Mirror DTOs from `src/api/types.rs` into `web/src/lib/types.ts`; update together at integration checkpoint. |
| Agents overwrite each other | High | Use separate worktrees and enforce file ownership. |
| Single binary packaging breaks during dev | Low | Defer embedding to Task 20 after Web UI build is stable. |

## Commands

Backend:

```bash
cargo fmt
cargo test
cargo build
cargo build --release
```

Frontend:

```bash
cd web
bun install
bun run build
```

Runtime examples:

```bash
cargo run -- serve --addr 127.0.0.1:4761
cargo run -- db sync
cargo run -- source list
cargo run -- sync run
```
