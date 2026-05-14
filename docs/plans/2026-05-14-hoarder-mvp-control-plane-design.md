# Hoarder MVP Control Plane Design

Date: 2026-05-14

## Objective

The next Hoarder phase turns the current foundation into a usable local sync
product. A user should be able to create a source, test it, create a sync job,
run it manually, inspect run details, view items and errors, save runtime
settings, and do the same core operations from the CLI. In `serve` mode,
enabled interval jobs should also run automatically from a single-process
scheduler.

This phase keeps the original first-release constraints:

- Local-first by default.
- One-way sync only: `source -> local vault`.
- No search.
- No bidirectional sync.
- No automatic local deletion.
- No authentication or multi-user remote deployment mode.
- OpenDAL remains the only implemented connector family.
- Scheduler support is fixed interval only; cron expressions are out of scope.

## Product Workflow

The first complete workflow is:

```text
Create source
  -> Test source
  -> Create job
  -> Run job manually
  -> Inspect run detail, synced items, and errors
  -> Let serve-mode scheduler run enabled interval jobs
```

The CLI and Web UI should expose the same underlying behavior. They should not
implement separate business logic.

## Architecture

The current architecture already has good core boundaries:

```text
CLI / Web UI
  -> command handlers / Axum API
  -> sync engine
  -> connector trait
  -> vault writer
  -> SeaORM repository
  -> SQLite
```

This phase adds a thin application service layer for orchestration:

```text
CLI / Axum routes
  -> app services
      source service
      job service
      run service
      settings service
      scheduler service
  -> repositories / sync engine / connector validation
```

Routes should only extract inputs, call services, and map results to DTOs.
CLI handlers should call the same services directly. Repositories should stay
focused on persistence. The sync engine remains a pure executor and should not
know about HTTP, CLI, or scheduler concerns.

Suggested module layout:

```text
src/app/mod.rs
src/app/source_service.rs
src/app/job_service.rs
src/app/run_service.rs
src/app/settings_service.rs
src/app/scheduler.rs
```

This is not a broad rewrite. Existing code can be moved incrementally when new
MVP behavior needs a clear shared boundary.

## Job Model

`sync_job` becomes the shared definition for manual runs and scheduled runs.

Recommended fields:

```text
id
source_id
name
enabled
schedule_kind                 "manual" | "interval"
schedule_interval_seconds     nullable integer
status                        "idle" | "running" | "paused" | "failed"
last_run_at                   nullable datetime
last_run_status               nullable string
last_run_id                   nullable uuid
cursor                        nullable string
created_at
updated_at
```

Rules:

- `manual` jobs never run automatically.
- `interval` jobs require `schedule_interval_seconds > 0`.
- Disabled jobs display as paused in the UI.
- A job in `running` state cannot be started again.
- A manual run and scheduler run use the same run path.
- The job row is updated after every run with the last run id, status, and time.

The first implementation should not add cron expressions. Fixed intervals are
enough for the local-first MVP and keep the scheduler state easy to reason
about.

## Run And Item APIs

Run listing remains a lightweight summary endpoint. Details are fetched by id.

```text
GET /api/runs
GET /api/runs/{id}
GET /api/items?sourceId=&status=&runId=
GET /api/errors?runId=&sourceId=
```

`GET /api/runs/{id}` returns one run plus related source and job display fields:

```json
{
  "id": "run-id",
  "jobId": "job-id",
  "sourceId": "source-id",
  "sourceName": "Local docs",
  "jobName": "Local docs interval",
  "status": "completed",
  "startedAt": "2026-05-14T10:00:00Z",
  "finishedAt": "2026-05-14T10:00:12Z",
  "durationMs": 12000,
  "counts": {
    "processed": 20,
    "synced": 3,
    "skipped": 17,
    "failed": 0,
    "deleted": 0
  },
  "errors": []
}
```

Items stay on their own endpoint. A large run can contain many items, and
embedding all of them inside run detail would make the response too heavy.

## Settings Model

Settings are split into boot-time config and runtime settings.

Boot-time config is loaded from JSON or CLI flags:

```text
database_path
vault_path
listen_addr
```

Runtime settings are persisted in `app_setting` and may be changed by API or
Web UI:

```text
job_concurrency
file_concurrency
log_level
```

For the MVP, `PATCH /api/settings` updates runtime settings only. It should not
pretend to live-edit `database_path`, `vault_path`, or `listen_addr`, because
those are process startup concerns. The API can still return those boot-time
values as read-only fields.

Recommended response:

```json
{
  "databasePath": "./hoarder.db",
  "vaultPath": "./vault",
  "listenAddr": "127.0.0.1:4761",
  "jobConcurrency": 1,
  "fileConcurrency": 4,
  "logLevel": "info",
  "readOnly": {
    "databasePath": true,
    "vaultPath": true,
    "listenAddr": true
  }
}
```

## Scheduler

The first scheduler is intentionally single-process. It runs only inside
`hoarder serve` and assumes one local Hoarder process owns the database.

Loop:

```text
Every scheduler tick:
  Load enabled interval jobs
  Skip jobs whose next run is not due
  Skip jobs whose status is running
  Start due jobs up to job_concurrency
  Use the same job runner as POST /api/jobs/{id}/run
```

Due calculation:

```text
due if last_run_at is null
due if now >= last_run_at + schedule_interval_seconds
```

The scheduler should not run when `job_concurrency == 0`; validation should
reject zero for persisted runtime settings.

This phase does not implement:

- Cross-process advisory locks.
- Distributed scheduling.
- Cron expressions.
- Missed-run backfill.
- Persistent next-run rows.

## Run Mutual Exclusion

Manual runs and scheduled runs need the same guard. Starting a run should:

1. Load the job.
2. Reject disabled jobs with validation error.
3. Reject `running` jobs with `409 CONFLICT`.
4. Mark job `running`.
5. Start the sync engine.
6. Finish the run.
7. Update job status and last run fields.

If the sync engine returns a connector-level error, the job should become
`failed`. If the run completes with item-level failures, the job should also
surface a failed last run status, while the run record keeps the detailed
counts.

## API Contract

New or changed endpoints:

```text
POST   /api/jobs
GET    /api/jobs
POST   /api/jobs/{id}/run
GET    /api/runs
GET    /api/runs/{id}
GET    /api/items?sourceId=&status=&runId=
GET    /api/errors?sourceId=&runId=
GET    /api/settings
PATCH  /api/settings
```

Request DTOs:

```json
// POST /api/jobs
{
  "sourceId": "source-id",
  "name": "Local docs every 5 minutes",
  "enabled": true,
  "schedule": {
    "kind": "interval",
    "intervalSeconds": 300
  }
}
```

```json
// PATCH /api/settings
{
  "jobConcurrency": 1,
  "fileConcurrency": 4,
  "logLevel": "info"
}
```

Error semantics:

- `400 BAD_REQUEST`: malformed JSON or path extraction errors.
- `404 NOT_FOUND`: source, job, or run does not exist.
- `409 CONFLICT`: run requested while job is already running.
- `422 UNPROCESSABLE_ENTITY`: valid JSON with invalid domain values, such as
  zero interval seconds.
- `500 INTERNAL_SERVER_ERROR`: internal failures with details hidden.

All errors continue to use the existing structured error shape.

## CLI Contract

The CLI should use explicit flags instead of requiring users to hand-write JSON.

Recommended commands:

```text
hoarder source list
hoarder source add --name "Local docs" --service fs --root ./docs
hoarder source test --id <source-id>

hoarder job list
hoarder job add --source-id <source-id> --name "Local docs" --interval 300

hoarder sync run --job-id <job-id>
hoarder sync status
hoarder sync status --job-id <job-id>
hoarder sync status --run-id <run-id>
```

The existing `--config-json` source add path may be kept as an escape hatch, but
the preferred UX should be typed OpenDAL flags.

Output should be stable, plain text, and script-friendly:

```text
ID                                   NAME         SERVICE  ENABLED  HEALTH
3f9b...                              Local docs   fs       true     healthy
```

JSON output can be added later; it is not required in this phase.

## Web Console

The Web console becomes fully live for the MVP workflow:

- Sources page:
  - create source
  - test source
  - show persisted health and last checked time
- Jobs page:
  - create manual or interval job
  - show status, interval, last run, next due time
  - trigger run now
  - handle `409` when already running
- Runs page:
  - list runs
  - select run
  - load detail by id
  - show filtered items and errors for the selected run
- Settings page:
  - save runtime settings
  - show boot-time fields as read-only

Mock fallback can remain for previewing when the API is unavailable, but once an
endpoint exists the live path must use it. Existing UI components should stay
compact and operational; no landing-page treatment is needed.

## Testing Strategy

Backend tests:

- Entity schema sync creates new job columns.
- Job creation validates interval schedules.
- Manual job creation works through API and repository.
- Running a disabled job is rejected.
- Running an already-running job returns `409`.
- Run detail returns source/job display fields and counts.
- Item listing filters by source, status, and run id.
- Settings patch persists runtime settings.
- Scheduler runs due interval jobs and skips manual or not-yet-due jobs.

Frontend tests:

- Creating a job updates the Jobs store.
- Triggering a run refreshes jobs and runs.
- Run detail selection loads errors/items.
- Settings save sends runtime-only fields.
- API mapping covers new job schedule and run detail DTOs.

Verification commands:

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
cd web
bun run verify
```

## Risks And Mitigations

| Risk | Impact | Mitigation |
| --- | --- | --- |
| Scheduler starts duplicate runs | High | Use shared job runner with a running-state guard and return `409` for concurrent starts. |
| Settings appear mutable but do not take effect | Medium | Clearly split boot-time config from runtime settings; mark boot fields read-only in API. |
| API and CLI drift apart | Medium | Route both through the same app services. |
| Web relies on stale mock shapes | Medium | Add API mapping tests for each new DTO and keep mock fallback secondary. |
| Job state stays `running` after crashes | Medium | Accept this for single-process MVP; add stale run recovery in a later phase. |

## Decisions

- Use fixed interval schedules only for MVP.
- Keep the scheduler single-process and local-first.
- Persist runtime settings in `app_setting`.
- Keep boot-time config read-only at runtime.
- Keep items and errors as separate list/filter endpoints rather than embedding
  all related rows in run detail.
- Add a thin app service layer to share logic between Axum, CLI, and scheduler.
