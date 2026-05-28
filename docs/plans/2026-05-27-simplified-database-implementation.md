# Simplified Database Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Replace the UUID/foreign-key-style SQLite model with a flat, indexed, local-integer schema.

**Status:** Completed on 2026-05-27 in `docs/simple-database-design`.

**Verification:** `cargo fmt --check`, `cargo clippy --all-targets --all-features`, `cargo test`, `cd web && bun run verify`, and `cargo build`.

**Architecture:** Keep the current table concepts, but remove SeaORM relationship fields and database foreign keys. Use local integer ID newtypes across Rust, explicit SQLite indexes in `sync_schema`, repository-level reference checks, and a bulk deletion mark keyed by `sync_item.last_run_id`.

**Tech Stack:** Rust 2024, SeaORM 2.0 entity-first, SQLite, Axum, Clap, Svelte 5, Bun.

---

## Prerequisites

- Work from the branch that contains `docs/plans/2026-05-27-simplified-database-design.md`.
- Treat this as a pre-1.0 breaking schema/API change. Do not preserve UUID database compatibility.
- Use @superpowers:test-driven-development for each task.
- Use @superpowers:verification-before-completion before reporting completion.

---

### Task 1: Convert Domain ID Newtypes To Local Integer IDs

**Files:**
- Modify: `src/core/types.rs`
- Modify: `tests/core_types.rs`

**Step 1: Write failing ID tests**

In `tests/core_types.rs`, remove the `uuid::Uuid` import and add integer ID expectations:

```rust
#[test]
fn core_types_local_ids_serialize_parse_and_display_as_numbers() {
    let source_id = SourceId::from_i64(42);

    assert_eq!(serde_json::to_value(source_id).unwrap(), json!(42));
    assert_eq!("42".parse::<SourceId>().unwrap(), source_id);
    assert_eq!(source_id.to_string(), "42");
    assert!("0".parse::<SourceId>().is_err());
    assert!("-1".parse::<SourceId>().is_err());
}
```

Update existing snapshot/ref tests to use `SourceId::from_i64(42)`.

**Step 2: Run the focused test and verify it fails**

Run: `cargo test core_types`

Expected: failures or compile errors because `SourceId::from_i64` does not exist and `from_uuid` is still used.

**Step 3: Replace the UUID newtype macro**

In `src/core/types.rs`, remove `uuid::Uuid` from the ID macro path and replace `uuid_newtype!` with a local integer macro:

```rust
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParseLocalIdError {
    value: String,
}

impl ParseLocalIdError {
    fn new(value: &str) -> Self {
        Self {
            value: value.to_owned(),
        }
    }
}

impl fmt::Display for ParseLocalIdError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "expected a positive integer id, got `{}`", self.value)
    }
}

impl std::error::Error for ParseLocalIdError {}

macro_rules! local_id_newtype {
    ($name:ident) => {
        #[derive(
            Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize,
        )]
        #[serde(transparent)]
        pub struct $name(i64);

        impl $name {
            pub fn new() -> Self {
                static NEXT: std::sync::atomic::AtomicI64 =
                    std::sync::atomic::AtomicI64::new(1);
                Self(NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed))
            }

            pub const fn from_i64(value: i64) -> Self {
                Self(value)
            }

            pub const fn as_i64(self) -> i64 {
                self.0
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl FromStr for $name {
            type Err = ParseLocalIdError;

            fn from_str(value: &str) -> Result<Self, Self::Err> {
                let parsed = value
                    .parse::<i64>()
                    .map_err(|_| ParseLocalIdError::new(value))?;
                if parsed <= 0 {
                    return Err(ParseLocalIdError::new(value));
                }

                Ok(Self(parsed))
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.0.fmt(formatter)
            }
        }
    };
}

local_id_newtype!(SourceId);
local_id_newtype!(JobId);
local_id_newtype!(RunId);
local_id_newtype!(ItemId);
```

Keep `new()` because sync-engine unit tests use generated fake IDs. Production repository code must use database-generated IDs after inserts.

**Step 4: Run the focused test and fix compile errors**

Run: `cargo test core_types`

Expected: PASS.

**Step 5: Commit**

```bash
git add src/core/types.rs tests/core_types.rs
git commit -m "refactor: use integer domain ids"
```

---

### Task 2: Flatten SeaORM Entities And Add Explicit SQLite Indexes

**Files:**
- Modify: `src/entity/source.rs`
- Modify: `src/entity/sync_job.rs`
- Modify: `src/entity/sync_run.rs`
- Modify: `src/entity/sync_item.rs`
- Modify: `src/entity/sync_error.rs`
- Modify: `src/db/schema.rs`
- Modify: `tests/db_schema.rs`

**Step 1: Write failing schema tests**

In `tests/db_schema.rs`, add helpers:

```rust
async fn foreign_keys(
    db: &impl ConnectionTrait,
    table_name: &str,
) -> Result<Vec<String>, sea_orm::DbErr> {
    let rows = db
        .query_all_raw(Statement::from_string(
            DatabaseBackend::Sqlite,
            format!("PRAGMA foreign_key_list({table_name})"),
        ))
        .await?;

    rows.into_iter()
        .map(|row| row.try_get::<String>("", "table"))
        .collect()
}

async fn index_names(
    db: &impl ConnectionTrait,
    table_name: &str,
) -> Result<BTreeSet<String>, sea_orm::DbErr> {
    let rows = db
        .query_all_raw(Statement::from_string(
            DatabaseBackend::Sqlite,
            format!("PRAGMA index_list({table_name})"),
        ))
        .await?;

    rows.into_iter()
        .map(|row| row.try_get::<String>("", "name"))
        .collect()
}
```

Extend `db_schema_syncs_expected_tables_and_job_columns` or add a new test:

```rust
#[tokio::test]
async fn db_schema_creates_flat_tables_without_foreign_keys_and_with_indexes()
-> Result<(), Box<dyn std::error::Error>> {
    let db = connect_sqlite("sqlite::memory:").await?;
    sync_schema(&db).await?;

    for table in ["source", "sync_job", "sync_run", "sync_item", "sync_error"] {
        assert!(
            foreign_keys(&db, table).await?.is_empty(),
            "{table} should not have database foreign keys"
        );
    }

    let sync_run_columns = table_columns(&db, "sync_run").await?;
    assert!(sync_run_columns.contains("source_name"));
    assert!(sync_run_columns.contains("job_name"));
    assert!(sync_run_columns.contains("deleted_count"));
    assert!(sync_run_columns.contains("bytes_written"));

    let sync_item_columns = table_columns(&db, "sync_item").await?;
    assert!(sync_item_columns.contains("last_run_id"));
    assert!(!sync_item_columns.contains("run_id"));

    let sync_error_columns = table_columns(&db, "sync_error").await?;
    assert!(!sync_error_columns.contains("item_id"));

    let sync_item_indexes = index_names(&db, "sync_item").await?;
    assert!(sync_item_indexes.contains("idx_sync_item_source_path"));
    assert!(sync_item_indexes.contains("idx_sync_item_source_last_run"));

    Ok(())
}
```

**Step 2: Run the schema test and verify it fails**

Run: `cargo test db_schema_creates_flat_tables_without_foreign_keys_and_with_indexes`

Expected: FAIL because existing entities still create relationship metadata and explicit indexes are not created.

**Step 3: Update entity primary keys and remove relation fields**

Use `i64` ids and remove all `belongs_to` / `has_many` fields from the five entity modules. Keep `app_setting` unchanged.

Example shape for `sync_job`:

```rust
#[sea_orm::model]
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "sync_job")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub source_id: i64,
    pub name: String,
    pub enabled: bool,
    pub schedule_kind: String,
    pub schedule_interval_seconds: Option<i64>,
    pub status: String,
    pub cursor: Option<String>,
    pub last_run_at: Option<DateTimeUtc>,
    pub last_run_status: Option<String>,
    pub last_run_id: Option<i64>,
    pub created_at: DateTimeUtc,
    pub updated_at: DateTimeUtc,
}
```

Apply the same pattern:

- `source.id: i64`, no relationship fields.
- `sync_run.id/job_id/source_id: i64`, add `source_name`, `job_name`, `deleted_count`, `bytes_written`.
- `sync_item.id/source_id: i64`, replace `run_id` with `last_run_id: Option<i64>`.
- `sync_error.id: i64`, ids become `Option<i64>` where nullable, remove `item_id`.

**Step 4: Add explicit index creation in schema sync**

Update `src/db/schema.rs` so `sync_schema` syncs entity tables and then creates indexes:

```rust
use sea_orm::{ConnectionTrait, DatabaseBackend, DatabaseConnection, Statement};

const INDEX_STATEMENTS: &[&str] = &[
    "CREATE INDEX IF NOT EXISTS idx_sync_job_source_id ON sync_job(source_id)",
    "CREATE INDEX IF NOT EXISTS idx_sync_job_enabled_schedule ON sync_job(enabled, schedule_kind, schedule_interval_seconds)",
    "CREATE INDEX IF NOT EXISTS idx_sync_run_started_at ON sync_run(started_at DESC)",
    "CREATE INDEX IF NOT EXISTS idx_sync_run_job_id ON sync_run(job_id)",
    "CREATE INDEX IF NOT EXISTS idx_sync_run_source_id ON sync_run(source_id)",
    "CREATE UNIQUE INDEX IF NOT EXISTS idx_sync_item_source_path ON sync_item(source_id, source_path)",
    "CREATE INDEX IF NOT EXISTS idx_sync_item_source_status_path ON sync_item(source_id, status, source_path)",
    "CREATE INDEX IF NOT EXISTS idx_sync_item_source_last_run ON sync_item(source_id, last_run_id)",
    "CREATE INDEX IF NOT EXISTS idx_sync_error_run_created ON sync_error(run_id, created_at DESC)",
    "CREATE INDEX IF NOT EXISTS idx_sync_error_source_created ON sync_error(source_id, created_at DESC)",
];

pub async fn sync_schema(db: &DatabaseConnection) -> AppResult<()> {
    db.get_schema_registry("hoarder::entity::*")
        .sync(db)
        .await
        .map_err(|error| AppError::Database(error.to_string()))?;

    for statement in INDEX_STATEMENTS {
        db.execute(Statement::from_string(
            DatabaseBackend::Sqlite,
            (*statement).to_owned(),
        ))
        .await
        .map_err(|error| AppError::Database(error.to_string()))?;
    }

    Ok(())
}
```

**Step 5: Run focused schema tests**

Run: `cargo test db_schema`

Expected: compile errors from repository code that still expects UUIDs are acceptable at this step only if the schema tests cannot compile. If compile errors block the test, continue immediately to Task 3 before committing.

**Step 6: Commit when the schema tests compile and pass**

```bash
git add src/entity src/db/schema.rs tests/db_schema.rs
git commit -m "refactor: flatten database schema"
```

---

### Task 3: Update Repository Source And Job Paths For Integer IDs

**Files:**
- Modify: `src/db/repository.rs`
- Modify: `src/app/source_service.rs`
- Modify: `src/app/job_service.rs`
- Modify: `tests/db_schema.rs`
- Modify: `tests/settings_repository.rs` only if compile errors require ID updates
- Modify: `tests/app_services.rs` only if compile errors require ID updates

**Step 1: Add repository-level reference validation tests**

In `tests/db_schema.rs`, add a test proving job creation no longer depends on database foreign keys:

```rust
#[tokio::test]
async fn repository_rejects_job_for_missing_source_in_application_code()
-> Result<(), Box<dyn std::error::Error>> {
    let db = connect_sqlite("sqlite::memory:").await?;
    sync_schema(&db).await?;
    let repository = SeaOrmRepository::new(db);

    let error = repository
        .create_job(NewSyncJob {
            source_id: SourceId::from_i64(999),
            name: "orphan job".to_owned(),
            enabled: true,
        })
        .await
        .expect_err("missing source should be rejected");

    assert!(
        error.to_string().contains("source not found"),
        "{error}"
    );

    Ok(())
}
```

**Step 2: Run the focused test and verify it fails or does not compile**

Run: `cargo test repository_rejects_job_for_missing_source_in_application_code`

Expected: FAIL/compile errors because repository code still uses `.as_uuid()` and UUID entity fields.

**Step 3: Update source repository mapping**

In `src/db/repository.rs`:

- Replace `SourceId::new().as_uuid()` on inserts with database-generated ids.
- Use `sea_orm::NotSet` for autoincrement primary keys.
- Convert model ids with `SourceId::from_i64(model.id)`.
- Convert query ids with `source_id.as_i64()`.

Example source insert shape:

```rust
let active_model = source::ActiveModel {
    id: sea_orm::NotSet,
    name: Set(input.name),
    kind: Set(connector_kind_to_str(input.kind).to_owned()),
    config_json: Set(input.config_json),
    enabled: Set(input.enabled),
    last_check_status: Set(None),
    last_checked_at: Set(None),
    created_at: Set(now),
    updated_at: Set(now),
};
let model = active_model.insert(&self.db).await.map_err(map_db_error)?;
```

**Step 4: Update job repository mapping and validation**

Before inserting a job in `create_scheduled_job`, check source existence:

```rust
source::Entity::find_by_id(input.source_id.as_i64())
    .one(&self.db)
    .await
    .map_err(map_db_error)?
    .ok_or_else(|| AppError::NotFound(format!("source not found: {}", input.source_id)))?;
```

Then insert `sync_job::ActiveModel` with `id: sea_orm::NotSet`, `source_id: Set(input.source_id.as_i64())`, and `last_run_id: Set(None)`.

Update all job record conversion:

```rust
id: JobId::from_i64(model.id),
source_id: SourceId::from_i64(model.source_id),
last_run_id: model.last_run_id.map(RunId::from_i64),
```

**Step 5: Update source and job services**

In `src/app/source_service.rs`, replace `source_id.as_uuid()` with `source_id.as_i64()` in `update_source_check`.

In `src/app/job_service.rs`:

- Replace `JobId::from_uuid`, `SourceId::from_uuid`, and `RunId::from_uuid` with integer constructors.
- Replace `job_id.as_uuid()` with `job_id.as_i64()`.
- Keep the atomic `update_many` claim in `mark_job_running`.

**Step 6: Run focused backend tests**

Run:

```bash
cargo test db_schema
cargo test app_services
```

Expected: PASS after updating any compile errors in tests.

**Step 7: Commit**

```bash
git add src/db/repository.rs src/app/source_service.rs src/app/job_service.rs tests/db_schema.rs tests/settings_repository.rs tests/app_services.rs
git commit -m "refactor: use integer ids in repositories"
```

---

### Task 4: Update Sync Repository And Engine Deletion Flow

**Files:**
- Modify: `src/sync/repository.rs`
- Modify: `src/sync/engine.rs`
- Modify: `src/db/repository.rs`
- Modify: `tests/sync_engine.rs`
- Modify: `tests/e2e_local_fs_sync.rs`
- Modify: `tests/sync_planner.rs` only if compile errors require ID updates

**Step 1: Update sync trait tests first**

In `tests/sync_engine.rs`, update `SyncJob` construction to include names:

```rust
SyncJob {
    id: job_id,
    source_id,
    source_name: "Test Source".to_owned(),
    job_name: "Test Job".to_owned(),
    connector_kind: ConnectorKind::OpenDal,
    connector_config: connector_config(),
    scan_cursor: None,
}
```

Replace `RepoEvent::MarkDeleted(String)` with a run-level event:

```rust
MarkMissingDeleted(RunId, SourceId),
```

Update the deletion test to expect `RepoEvent::MarkMissingDeleted(run_id, source_id)`.

**Step 2: Run sync engine tests and verify failure**

Run: `cargo test sync_engine`

Expected: FAIL/compile errors because the trait still has `known_item_states` and `mark_deleted`.

**Step 3: Change the sync repository trait**

In `src/sync/repository.rs`, update `SyncRepository`:

```rust
pub trait SyncRepository: Send + Sync {
    fn load_job(&self, job_id: JobId) -> RepositoryFuture<'_, SyncJob>;
    fn start_run<'a>(&'a self, job: &'a SyncJob) -> RepositoryFuture<'a, RunId>;
    fn item_state<'a>(
        &'a self,
        source_id: SourceId,
        source_path: &'a str,
    ) -> RepositoryFuture<'a, Option<StoredItemState>>;
    fn record_item_outcome(
        &self,
        run_id: RunId,
        outcome: ItemSyncOutcome,
    ) -> RepositoryFuture<'_, ()>;
    fn mark_missing_items_deleted(
        &self,
        run_id: RunId,
        source_id: SourceId,
    ) -> RepositoryFuture<'_, u64>;
    fn finish_run(
        &self,
        run_id: RunId,
        status: SyncRunStatus,
        summary: SyncRunSummary,
    ) -> RepositoryFuture<'_, ()>;
}
```

Remove `known_item_states` and `mark_deleted`.

**Step 4: Update `SyncJob` and engine deletion flow**

In `src/sync/engine.rs`, add names to `SyncJob`:

```rust
pub struct SyncJob {
    pub id: JobId,
    pub source_id: SourceId,
    pub source_name: String,
    pub job_name: String,
    pub connector_kind: ConnectorKind,
    pub connector_config: ConnectorConfig,
    pub scan_cursor: Option<String>,
}
```

Remove the post-scan loop over `known_item_states` and replace it with:

```rust
self.repository
    .mark_missing_items_deleted(run_id, job.source_id)
    .await
    .map_err(|source| SyncRunError::new(source, summary.clone()))?;
```

Keep `seen_paths` only if another path still needs it. If nothing uses it after this change, delete the `BTreeSet` import and `seen_paths` variable.

**Step 5: Implement integer sync repository operations**

In `src/db/repository.rs`:

- `load_job` uses integer ids and fills `source_name` / `job_name`.
- `start_run` inserts with `id: NotSet`, `job_name`, `source_name`, `deleted_count: Set(0)`, `bytes_written: Set(0)`, then returns `RunId::from_i64(model.id)`.
- `item_state` filters `SourceId.as_i64()` and `DeletedOnSourceAt.is_null()`.
- `record_item_outcome` writes `sync_item.last_run_id = run_id.as_i64()`.
- `insert_sync_error` no longer needs `item_id`.
- `finish_run` writes `deleted_count` and `bytes_written`.

For `mark_missing_items_deleted`, use `update_many`:

```rust
let now = Utc::now();
let result = sync_item::Entity::update_many()
    .col_expr(
        sync_item::Column::Status,
        sea_orm::sea_query::Expr::value(sync_status_to_str(SyncStatus::DeletedOnSource)),
    )
    .col_expr(
        sync_item::Column::DeletedOnSourceAt,
        sea_orm::sea_query::Expr::value(now),
    )
    .col_expr(
        sync_item::Column::LastRunId,
        sea_orm::sea_query::Expr::value(run_id.as_i64()),
    )
    .col_expr(
        sync_item::Column::UpdatedAt,
        sea_orm::sea_query::Expr::value(now),
    )
    .filter(sync_item::Column::SourceId.eq(source_id.as_i64()))
    .filter(sync_item::Column::DeletedOnSourceAt.is_null())
    .filter(
        sea_orm::sea_query::Expr::col(sync_item::Column::LastRunId)
            .is_null()
            .or(sea_orm::sea_query::Expr::col(sync_item::Column::LastRunId).ne(run_id.as_i64())),
    )
    .exec(&self.db)
    .await
    .map_err(map_db_error)?;

Ok(result.rows_affected)
```

If SeaORM expression typing is awkward, use `Condition::any()` with `ColumnTrait` filters instead of raw `Expr::col`.

**Step 6: Update e2e expectations**

In `tests/e2e_local_fs_sync.rs`:

- Replace direct UUID assertions with integer ids.
- Query `sync_item::Column::LastRunId` instead of `RunId`.
- Assert deleted rows get `last_run_id == Some(third.run_id.as_i64())`.

**Step 7: Run sync tests**

Run:

```bash
cargo test sync_engine
cargo test e2e_local_fs_sync
```

Expected: PASS.

**Step 8: Commit**

```bash
git add src/sync/repository.rs src/sync/engine.rs src/db/repository.rs tests/sync_engine.rs tests/e2e_local_fs_sync.rs tests/sync_planner.rs
git commit -m "refactor: simplify sync item persistence"
```

---

### Task 5: Update API, CLI, And OpenAPI For Integer IDs

**Files:**
- Modify: `src/api/types.rs`
- Modify: `src/api/openapi.rs`
- Modify: `src/api/routes.rs` only if compile errors require route type updates
- Modify: `src/cli.rs`
- Modify: `tests/api_routes.rs`
- Modify: `tests/cli_parse.rs`
- Modify: `tests/cli_commands.rs`
- Modify: `tests/api_error.rs` only if error text changes

**Step 1: Update API route tests first**

In `tests/api_routes.rs`, change UUID-looking expected JSON values to numbers. For new rows, assert numeric shape instead of exact ID unless the test controls inserted IDs:

```rust
assert!(response.body["id"].is_number());
assert!(response.body["sourceId"].is_number());
```

For route paths, use `job.id.to_string()` and `run_id.to_string()` as today. They will now produce `1`, `2`, etc.

**Step 2: Update CLI parse tests**

In `tests/cli_parse.rs`, replace string ids:

```rust
let test = Cli::parse_from(["hoarder", "source", "test", "--id", "1"]);
// ...
let add = Cli::parse_from([
    "hoarder",
    "job",
    "add",
    "--source-id",
    "1",
    "--name",
    "Every five minutes",
    "--interval",
    "300",
]);
let run = Cli::parse_from(["hoarder", "sync", "run", "--job-id", "1"]);
```

**Step 3: Run API/CLI tests and verify failure**

Run:

```bash
cargo test api_routes
cargo test cli_parse
cargo test cli_commands
```

Expected: FAIL/compile errors until all UUID assumptions are removed.

**Step 4: Update DTO types only where needed**

Rust DTO fields can continue to use `SourceId`, `JobId`, `RunId`, and `ItemId`; serialization changes to numbers through Task 1.

Update `SyncErrorDto.id` from `String` to `i64` or `ItemId`-style wrapper. Prefer `i64` for log rows if no typed `ErrorId` exists:

```rust
pub struct SyncErrorDto {
    pub id: i64,
    pub run_id: Option<RunId>,
    pub source_id: Option<SourceId>,
    pub source_path: Option<String>,
    pub code: String,
    pub message: String,
    pub created_at: Option<DateTime<Utc>>,
}
```

**Step 5: Update OpenAPI schemas**

In `src/api/openapi.rs`, replace UUID helper usage for local IDs with integer schemas:

```rust
fn local_id_schema() -> Value {
    json!({"type": "integer", "minimum": 1})
}

fn nullable_local_id_schema() -> Value {
    json!({"type": ["integer", "null"], "minimum": 1})
}
```

Use `local_id_schema()` for `id`, `sourceId`, `jobId`, `runId`, and `itemId` fields. Keep UUID schema only if a non-local identifier remains.

**Step 6: Update CLI validation expectations**

`parse_id<T>` can stay generic. The new `FromStr` implementation will reject `0`, negative numbers, and non-numeric strings with a domain-specific validation error.

**Step 7: Run focused tests**

Run:

```bash
cargo test api_routes
cargo test cli_parse
cargo test cli_commands
cargo test api_error
```

Expected: PASS.

**Step 8: Commit**

```bash
git add src/api src/cli.rs tests/api_routes.rs tests/cli_parse.rs tests/cli_commands.rs tests/api_error.rs
git commit -m "refactor: expose local integer ids"
```

---

### Task 6: Update Frontend Types, Mocks, And API Mapping

**Files:**
- Modify: `web/src/lib/types.ts`
- Modify: `web/src/lib/api.ts`
- Modify: `web/src/lib/state.ts`
- Modify: `web/src/components/JobForm.svelte`
- Modify: `web/src/routes/Sources.svelte`
- Modify: `web/src/routes/Jobs.svelte`
- Modify: `web/src/routes/Runs.svelte`
- Modify: `web/src/routes/Overview.svelte` only if TypeScript points to ID typing issues

**Step 1: Run frontend type check to capture current baseline**

Run: `cd web && bun run verify`

Expected: It may fail after backend type changes are not yet reflected in generated assumptions. Use the output to update all string-id references.

**Step 2: Define a local ID alias**

In `web/src/lib/types.ts`, add:

```ts
export type LocalId = number;
```

Change DTO IDs:

```ts
export interface SourceDto {
  id: LocalId;
  // ...
}

export interface SyncJobDto {
  id: LocalId;
  sourceId: LocalId;
  lastRunId?: LocalId;
  // ...
}

export interface JobFormInput {
  sourceId: LocalId;
  // ...
}
```

Apply the same to run, item, error, filters, and callback props.

**Step 3: Update mock data**

In `web/src/lib/api.ts`, replace mock string IDs with numbers:

```ts
const mockSources: SourceDto[] = [
  { id: 1, name: "Local notes", /* existing fields */ },
];

const mockJobs: SyncJobDto[] = [
  { id: 1, sourceId: 1, /* existing fields */ },
];
```

Keep human-readable names in `name` fields, not IDs.

**Step 4: Update backend DTO interfaces**

In `web/src/lib/api.ts`, change backend id fields from `string` to `number`:

```ts
interface BackendSourceDto {
  id: number;
  // ...
}

interface BackendJobRunResponse {
  runId: number;
  status: SyncRunDto["status"] | "pending" | "synced" | "failed" | "skipped" | "deleted_on_source";
}
```

Update maps from `Map<string, ...>` to `Map<LocalId, ...>`.

**Step 5: Update Svelte callback prop types**

For example:

```svelte
let {
  sources,
  onTestSource,
}: {
  sources: Loadable<SourceDto[]>;
  onTestSource: (sourceId: LocalId) => Promise<void> | void;
} = $props();
```

In `JobForm.svelte`, keep select binding numeric. If Svelte coerces DOM option values to strings, convert at submit time:

```ts
let sourceId = $state<number | undefined>(undefined);

function submit() {
  if (!sourceId || !name.trim()) {
    return;
  }
  onSubmit({
    sourceId,
    name: name.trim(),
    enabled,
    schedule: scheduleValue(),
  });
}
```

**Step 6: Run frontend verification**

Run: `cd web && bun run verify`

Expected: PASS.

**Step 7: Commit**

```bash
git add web/src
git commit -m "refactor: use numeric ids in console"
```

---

### Task 7: Update Remaining Backend Tests And Remove UUID Assumptions

**Files:**
- Modify any remaining files reported by `rg "from_uuid|as_uuid|Uuid|uuid_schema|source-1|job-1|run-" src tests web/src`
- Modify: `Cargo.toml` only if `uuid` is no longer used outside tests/temp names

**Step 1: Search for UUID-specific code**

Run:

```bash
rg -n "from_uuid|as_uuid|Uuid|uuid_schema|source-1|job-1|run-" src tests web/src
```

Expected: only unrelated temp directory UUID usage may remain.

**Step 2: Fix each remaining production occurrence**

Rules:

- `from_uuid(...)` becomes `from_i64(...)`.
- `.as_uuid()` becomes `.as_i64()`.
- `Uuid::new_v4()` for primary keys disappears from production database inserts.
- OpenAPI `uuid_schema()` remains only if a real UUID field still exists.
- Test fixture string ids become positive integers.

**Step 3: Decide whether to keep the `uuid` dependency**

If `uuid` remains only for temp directory names in tests, either:

- Keep the dependency to minimize churn, or
- Replace temp suffixes with integer timestamps and remove `uuid`.

Prefer keeping it unless dependency cleanup is explicitly in scope.

**Step 4: Run broad backend tests**

Run:

```bash
cargo test
```

Expected: PASS.

**Step 5: Commit**

```bash
git add src tests Cargo.toml
git commit -m "test: update id coverage"
```

Skip this commit if Task 7 finds no changes.

---

### Task 8: Final Verification And Documentation Updates

**Files:**
- Modify: `docs/development.md` if CLI examples mention UUID IDs
- Modify: `docs/plans/2026-05-27-hoarder-plan-technical-roadmap.md` if its database section still claims UUID or relationship-heavy schema
- Modify: `README.md` if command examples mention UUID IDs

**Step 1: Update docs**

Search:

```bash
rg -n "UUID|uuid|job-id|source-id|run-id|foreign key|SeaORM entity" README.md docs
```

Update examples to use integer IDs:

```bash
hoarder sync run --job-id 1
hoarder sync status --run-id 1
```

**Step 2: Run format and lint**

Run:

```bash
cargo fmt --check
cargo clippy --all-targets --all-features
```

Expected: PASS.

If `cargo fmt --check` fails, run `cargo fmt`, then rerun `cargo fmt --check`.

**Step 3: Run full test suites**

Run:

```bash
cargo test
cd web && bun run verify
```

Expected: PASS.

**Step 4: Build frontend and backend if UI files changed**

Run:

```bash
cd web && bun run build
cargo build
```

Expected: PASS.

**Step 5: Commit docs or formatting changes**

If Step 1 or formatting changed files:

```bash
git add README.md docs src tests web/src web/dist
git commit -m "docs: update simplified database workflow"
```

Adjust the staged paths to actual changed files. Do not stage unrelated user changes.

**Step 6: Final status check**

Run:

```bash
git status --short
git log --oneline -6
```

Expected: no unstaged/untracked implementation changes except intentionally generated artifacts that are documented.

---

## Execution Notes

- Prefer integer `i64` in Rust entity columns because SQLite `INTEGER` is signed 64-bit.
- Use typed wrappers at Rust/API boundaries rather than passing raw `i64` everywhere.
- Do not introduce database foreign keys in SeaORM relations or raw SQL.
- Keep `sync_error` independent from `sync_item`; use `source_path` for item-level error context.
- Do not add `sync_item_event` in this implementation. The accepted design keeps `sync_item` as the current index and stores run-level history in `sync_run` plus errors in `sync_error`.
