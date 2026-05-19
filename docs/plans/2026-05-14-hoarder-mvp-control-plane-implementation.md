# Hoarder MVP Control Plane Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Complete the local MVP control plane: source testing, job creation, manual runs, fixed-interval scheduling, run detail, filtered items/errors, runtime settings, and real CLI/Web workflows.

**Architecture:** Add a thin app service layer that is shared by Axum routes, CLI handlers, and the scheduler. Extend SeaORM entity-first models for job scheduling and runtime settings, keep sync execution inside the existing sync engine, and keep routes as extract/map glue.

**Tech Stack:** Rust 2024, Axum, Tokio, SeaORM 2.0 entity-first, SQLite, Clap, Svelte 5, Tailwind CSS 4, Bun, strict Clippy.

---

## Coordination Rules

- Always use the strict Clippy gate:
  `cargo clippy --all-targets --all-features -- -D warnings`
- Use Bun for every frontend command.
- Keep commits small and lane-specific.
- Do not rewrite unrelated files or reformat files owned by another lane.
- Do not change the one-way sync policy.
- Do not add search, bidirectional sync, auth, cron, or automatic local deletion.
- Prefer Rust/Axum boundaries: routes extract inputs, services own orchestration,
  repositories persist data.

## Suggested Worktrees

Create worktrees after Task 1 lands:

```bash
git worktree add ../hoarder-mvp-db feature/mvp-db-repositories
git worktree add ../hoarder-mvp-api feature/mvp-api-services
git worktree add ../hoarder-mvp-scheduler feature/mvp-scheduler
git worktree add ../hoarder-mvp-cli feature/mvp-cli
git worktree add ../hoarder-mvp-web feature/mvp-web
```

Merge order:

```text
1. contract-foundation
2. db-repositories
3. api-services
4. cli
5. web
6. scheduler
7. integration
```

## Task 1: Shared MVP Contracts

**Description:** Define shared DTOs and domain enums for job schedules, job status, run detail, item filters, error filters, and runtime settings. This task must land before parallel implementation.

**Files:**

- Modify: `src/api/types.rs`
- Modify: `src/core/types.rs`
- Modify: `web/src/lib/types.ts`
- Test: `tests/core_types.rs`

**Acceptance criteria:**

- Job schedule supports `manual` and `interval`.
- Interval schedule validates positive seconds at the service/API boundary later.
- Job status values are stable strings: `idle`, `running`, `paused`, `failed`.
- Runtime settings distinguish mutable fields from read-only boot config fields.
- Rust and TypeScript names match API camelCase responses.

**Verification:**

```bash
cargo test core_types
cargo fmt --check
cd web
bun run verify
```

**Dependencies:** None

**Estimated scope:** Medium

**Commit:**

```bash
git add src/api/types.rs src/core/types.rs web/src/lib/types.ts tests/core_types.rs
git commit -m "feat: add mvp control plane contracts"
```

## Task 2: Job Entity And Repository Fields

**Description:** Extend the `sync_job` entity and repository records to store schedule, status, and last-run metadata.

**Files:**

- Modify: `src/entity/sync_job.rs`
- Modify: `src/db/repository.rs`
- Test: `tests/db_schema.rs`

**Acceptance criteria:**

- Schema sync creates schedule and last-run columns.
- New jobs default to `manual`, `idle`, and no last run.
- Interval jobs can be inserted and listed with interval seconds.
- Existing repository tests still pass.

**Verification:**

```bash
cargo test db_schema
cargo test api_routes
cargo clippy --all-targets --all-features -- -D warnings
```

**Dependencies:** Task 1

**Estimated scope:** Medium

**Commit:**

```bash
git add src/entity/sync_job.rs src/db/repository.rs tests/db_schema.rs
git commit -m "feat: persist sync job schedules"
```

## Task 3: Runtime Settings Repository

**Description:** Use `app_setting` for mutable runtime settings and merge them with boot-time config for API output.

**Files:**

- Modify: `src/entity/app_setting.rs` only if schema adjustment is needed
- Modify: `src/db/repository.rs`
- Modify: `src/config.rs`
- Test: `tests/db_schema.rs`
- Test: create `tests/settings_repository.rs`

**Acceptance criteria:**

- Runtime settings can be loaded from an empty DB with config defaults.
- Runtime settings can be patched and reloaded.
- `job_concurrency` and `file_concurrency` reject zero.
- Boot-time paths and listen address remain read-only.

**Verification:**

```bash
cargo test settings_repository
cargo test db_schema
cargo clippy --all-targets --all-features -- -D warnings
```

**Dependencies:** Task 1

**Estimated scope:** Medium

**Commit:**

```bash
git add src/entity/app_setting.rs src/db/repository.rs src/config.rs tests/db_schema.rs tests/settings_repository.rs
git commit -m "feat: persist runtime settings"
```

## Task 4: App Service Layer

**Description:** Add shared services for source testing, job creation, job running, run detail, item/error filters, and settings mutation.

**Files:**

- Create: `src/app/mod.rs`
- Create: `src/app/source_service.rs`
- Create: `src/app/job_service.rs`
- Create: `src/app/run_service.rs`
- Create: `src/app/settings_service.rs`
- Modify: `src/lib.rs`
- Modify: `src/db/repository.rs`
- Test: create `tests/app_services.rs`

**Acceptance criteria:**

- Source test service validates a connector and persists health.
- Job service creates manual and interval jobs with validation.
- Job runner rejects disabled jobs.
- Job runner rejects `running` jobs with a conflict-style app error.
- Job runner updates last-run fields after completion.
- Run service returns one run detail with source/job display fields.
- Item/error filters work without API involvement.

**Verification:**

```bash
cargo test app_services
cargo test sync_engine
cargo clippy --all-targets --all-features -- -D warnings
```

**Dependencies:** Tasks 2 and 3

**Estimated scope:** Large; split into two commits if it grows beyond one focused session.

**Commit:**

```bash
git add src/app src/lib.rs src/db/repository.rs tests/app_services.rs
git commit -m "feat: add control plane services"
```

## Task 5: API Endpoints

**Description:** Wire the MVP services into Axum routes and keep route handlers thin.

**Files:**

- Modify: `src/api/routes.rs`
- Modify: `src/api/types.rs`
- Modify: `src/api/error.rs`
- Modify: `src/api/state.rs`
- Test: `tests/api_routes.rs`
- Test: `tests/api_error.rs`

**Acceptance criteria:**

- `POST /api/jobs` creates manual and interval jobs.
- `GET /api/jobs` includes schedule, status, and last-run fields.
- `POST /api/jobs/{id}/run` uses the shared job runner.
- Running an already-running job returns `409`.
- `GET /api/runs/{id}` returns run detail.
- `GET /api/items` supports `sourceId`, `status`, and `runId` filters.
- `GET /api/errors` supports `sourceId` and `runId` filters.
- `PATCH /api/settings` persists runtime settings only.

**Verification:**

```bash
cargo test api_routes
cargo test api_error
cargo clippy --all-targets --all-features -- -D warnings
```

**Dependencies:** Task 4

**Estimated scope:** Medium

**Commit:**

```bash
git add src/api tests/api_routes.rs tests/api_error.rs
git commit -m "feat: expose mvp control plane api"
```

## Task 6: CLI Command Execution

**Description:** Turn existing parsed CLI commands into real operations and add job commands.

**Files:**

- Modify: `src/cli.rs`
- Modify: `src/main.rs`
- Modify: `src/server.rs` if shared database/config helpers need public access
- Test: `tests/cli_parse.rs`
- Test: create `tests/cli_commands.rs`

**Acceptance criteria:**

- `source list` prints persisted sources.
- `source add --name ... --service fs --root ...` creates an OpenDAL fs source.
- `source test --id ...` validates and records source health.
- `job list` prints jobs.
- `job add --source-id ... --name ... --interval 300` creates an interval job.
- `sync run --job-id ...` runs a job through the shared runner.
- `sync status`, `sync status --job-id`, and `sync status --run-id` print useful summaries.
- Existing `--config` support works for commands that touch the DB.

**Verification:**

```bash
cargo test cli_parse
cargo test cli_commands
cargo clippy --all-targets --all-features -- -D warnings
```

**Dependencies:** Task 4

**Estimated scope:** Medium

**Commit:**

```bash
git add src/cli.rs src/main.rs src/server.rs tests/cli_parse.rs tests/cli_commands.rs
git commit -m "feat: execute source job and sync cli commands"
```

## Task 7: Scheduler

**Description:** Add a single-process fixed-interval scheduler to `serve` mode.

**Files:**

- Create: `src/app/scheduler.rs`
- Modify: `src/server.rs`
- Modify: `src/api/state.rs` if runtime settings are shared through state
- Test: create `tests/scheduler.rs`

**Acceptance criteria:**

- Scheduler starts with `serve`.
- Manual jobs are ignored by scheduler.
- Disabled jobs are ignored.
- Interval jobs run when due.
- Not-yet-due interval jobs are skipped.
- `running` jobs are skipped.
- Scheduler uses the same shared job runner as the API and CLI.
- Scheduler honors `job_concurrency`.

**Verification:**

```bash
cargo test scheduler
cargo test app_services
cargo clippy --all-targets --all-features -- -D warnings
```

**Dependencies:** Tasks 4 and 5

**Estimated scope:** Medium

**Commit:**

```bash
git add src/app/scheduler.rs src/server.rs src/api/state.rs tests/scheduler.rs
git commit -m "feat: run interval jobs in serve mode"
```

## Task 8: Frontend API Client And State

**Description:** Update the Svelte data layer to use the new live API contracts.

**Files:**

- Modify: `web/src/lib/types.ts`
- Modify: `web/src/lib/api.ts`
- Modify: `web/src/lib/state.ts`
- Test: `web/tests/state.test.ts`

**Acceptance criteria:**

- `createJob` calls `POST /api/jobs`.
- `runJob` refreshes the job/run state using live response data.
- `getRunDetail` calls `GET /api/runs/{id}`.
- `getItems` supports `runId`, `sourceId`, and `status`.
- `getErrors` supports `runId` and `sourceId`.
- `updateSettings` sends runtime-only mutable fields.
- Mock fallback matches the new DTO shapes.

**Verification:**

```bash
cd web
bun run verify
```

**Dependencies:** Tasks 1 and 5

**Estimated scope:** Medium

**Commit:**

```bash
git add web/src/lib/types.ts web/src/lib/api.ts web/src/lib/state.ts web/tests/state.test.ts
git commit -m "feat: wire frontend data layer to mvp api"
```

## Task 9: Frontend MVP Workflow UI

**Description:** Add UI for job creation, run detail loading, filtered items/errors, and settings save behavior.

**Files:**

- Modify: `web/src/routes/Jobs.svelte`
- Modify: `web/src/routes/Runs.svelte`
- Modify: `web/src/routes/Settings.svelte`
- Modify: `web/src/App.svelte` if props/events need adjustment
- Create: `web/src/components/JobForm.svelte`
- Create: `web/src/components/RunDetailPanel.svelte`
- Test: `web/tests/state.test.ts`

**Acceptance criteria:**

- Jobs page creates manual and interval jobs.
- Jobs page shows schedule kind, interval, last run, next due, and status.
- Runs page loads selected run detail.
- Runs page displays selected run errors and filtered item counts.
- Settings page saves runtime settings and renders boot fields as read-only.
- UI remains compact and operational.

**Verification:**

```bash
cd web
bun run verify
```

**Dependencies:** Task 8

**Estimated scope:** Medium

**Commit:**

```bash
git add web/src/routes/Jobs.svelte web/src/routes/Runs.svelte web/src/routes/Settings.svelte web/src/App.svelte web/src/components/JobForm.svelte web/src/components/RunDetailPanel.svelte web/tests/state.test.ts
git commit -m "feat: add mvp control plane web workflow"
```

## Task 10: Integration And Documentation

**Description:** Run the full system checks, update README checklists, and document remaining risks.

**Files:**

- Modify: `README.md`
- Modify: `README.zh-CN.md`
- Modify: `docs/development.md`
- Test: update any integration test needed for the new workflow

**Acceptance criteria:**

- README feature checklist reflects completed API, CLI, scheduler, and Web work.
- Development docs describe the full local MVP workflow.
- Existing source-test persistence still works.
- E2E local fs sync still passes.
- Release build works after web build.

**Verification:**

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
cd web
bun run verify
cd ..
cargo build --release
```

**Dependencies:** Tasks 1-9

**Estimated scope:** Small

**Commit:**

```bash
git add README.md README.zh-CN.md docs/development.md tests
git commit -m "docs: update mvp control plane workflow"
```

## Parallel Agent Prompts

Use these prompts after Task 1 is merged into `main`.

### DB And Repository Agent

```text
Implement Tasks 2-3 from docs/plans/2026-05-14-hoarder-mvp-control-plane-implementation.md.

Ownership:
- src/entity/sync_job.rs
- src/entity/app_setting.rs
- src/db/repository.rs
- src/config.rs
- tests/db_schema.rs
- tests/settings_repository.rs

You are not alone in the codebase. Do not revert edits made by other agents.
Do not touch API routes, CLI, scheduler, or web files unless compilation is
blocked by a shared contract; report contract blockers instead of broad edits.

Use SeaORM 2.0 entity-first. Keep strict Clippy:
cargo clippy --all-targets --all-features -- -D warnings

Run:
- cargo test db_schema
- cargo test settings_repository
- cargo clippy --all-targets --all-features -- -D warnings

Commit only your lane changes. Return changed files, test output summary, and
any repository contract decisions.
```

### App Services And API Agent

```text
Implement Tasks 4-5 from docs/plans/2026-05-14-hoarder-mvp-control-plane-implementation.md.

Ownership:
- src/app/**
- src/lib.rs
- src/api/**
- tests/app_services.rs
- tests/api_routes.rs
- tests/api_error.rs

You are not alone in the codebase. Do not revert edits made by other agents.
Keep routes thin. Shared business flow belongs in services. Do not modify web
files or CLI command syntax beyond what is required by shared contracts.

Run:
- cargo test app_services
- cargo test api_routes
- cargo test api_error
- cargo clippy --all-targets --all-features -- -D warnings

Commit only your lane changes. Return endpoint list, changed files, and
verification summary.
```

### CLI Agent

```text
Implement Task 6 from docs/plans/2026-05-14-hoarder-mvp-control-plane-implementation.md.

Ownership:
- src/cli.rs
- src/main.rs
- src/server.rs only for shared config/database helper access
- tests/cli_parse.rs
- tests/cli_commands.rs

You are not alone in the codebase. Do not revert edits made by other agents.
The CLI must call shared app services, not duplicate route logic. Prefer typed
OpenDAL flags for source add, with fs support first.

Run:
- cargo test cli_parse
- cargo test cli_commands
- cargo clippy --all-targets --all-features -- -D warnings

Commit only your lane changes. Return command examples and verification
summary.
```

### Scheduler Agent

```text
Implement Task 7 from docs/plans/2026-05-14-hoarder-mvp-control-plane-implementation.md.

Ownership:
- src/app/scheduler.rs
- scheduler integration in src/server.rs
- src/api/state.rs only if needed for shared runtime settings
- tests/scheduler.rs

You are not alone in the codebase. Do not revert edits made by other agents.
Scheduler is single-process, fixed interval only. Use the shared job runner.
Do not add cron, distributed locks, or backfill behavior.

Run:
- cargo test scheduler
- cargo test app_services
- cargo clippy --all-targets --all-features -- -D warnings

Commit only your lane changes. Return scheduler behavior and verification
summary.
```

### Web Agent

```text
Implement Tasks 8-9 from docs/plans/2026-05-14-hoarder-mvp-control-plane-implementation.md.

Ownership:
- web/**

You are not alone in the codebase. Do not revert edits made by other agents.
Use Bun. Keep the UI compact and operational, not a landing page. Do not change
backend files.

Run:
- cd web
- bun run verify

Commit only your lane changes. Return changed files, behavior summary, and
verification summary.
```

### Integration Agent

```text
Implement Task 10 from docs/plans/2026-05-14-hoarder-mvp-control-plane-implementation.md after all MVP lanes are merged.

Ownership:
- README.md
- README.zh-CN.md
- docs/development.md
- integration-only tests/fixes needed for the full MVP workflow

You are not alone in the codebase. Do not redesign already merged subsystems.
Keep strict Clippy and Bun verification.

Run:
- cargo fmt --check
- cargo clippy --all-targets --all-features -- -D warnings
- cargo test
- cd web
- bun run verify
- cd ..
- cargo build --release

Commit only integration/doc changes. Return full verification results and
remaining risks.
```

## Checkpoints

### Checkpoint 1: Contract Foundation

- [x] Task 1 merged.
- [x] Rust and TypeScript contract names align.
- [x] No implementation lane has started from stale contracts.

### Checkpoint 2: Backend Control Plane

- [x] Tasks 2-5 merged.
- [x] API can create jobs, run jobs, fetch run detail, filter items/errors, and patch settings.
- [x] Strict Clippy passes.

### Checkpoint 3: Interfaces

- [x] Tasks 6, 8, and 9 merged.
- [x] CLI and Web both use the shared MVP workflow.
- [x] Bun verification passes.

### Checkpoint 4: Scheduler And Release Readiness

- [x] Task 7 merged.
- [x] Task 10 merged.
- [x] Full verification passes.
- [x] README checklists match actual behavior.

## Implementation Notes

- Runtime settings persist the desired tracing filter and hot-reload it after a
  successful update.

## Open Questions For Later Phases

- Should stale `running` jobs be recovered automatically after a process crash?
- Should CLI add `--json` output for automation?
- Should Web remove mock fallback entirely once API coverage is complete?
- Should explicit schema migrations replace schema sync before wider release?
