# Hoarder Simplified Database Design

Date: 2026-05-27
Status: Accepted
Implementation: Completed on 2026-05-27 in `docs/simple-database-design`
Scope: SQLite schema, persistence model, sync write path, API/CLI ID shape

## Context

The current database model uses SeaORM entity relationships to describe source, job, run, item, and error links. That makes the local schema look like an application domain graph, but Hoarder is a local-first sync index. The database should optimize for fast append/update operations, simple migrations, and clear query paths, not for enforcing a relational business model.

The main friction is not the number of tables. The friction is that tables are strongly coupled through database-level relationships and UUID identifiers even where local integer identifiers are enough.

## Decision

Use a flat SQLite schema with no database foreign keys. Tables keep ordinary reference columns such as `source_id`, `job_id`, `run_id`, and `item_id`, but those columns are not foreign keys.

Use `INTEGER PRIMARY KEY AUTOINCREMENT` for local entities and logs. Keep `app_setting.key` as a text primary key.

The repository layer owns reference validation:

- Creating a job validates that its source exists.
- Starting a run validates that its job and source can be loaded.
- Recording item outcomes validates only what the sync engine needs to proceed.
- Query endpoints tolerate missing soft references where historical rows should remain readable.

SQLite owns only local row identity, uniqueness, and query performance through indexes.

## Table Model

### `source`

Stores connector configuration and source health state.

| Column | Type | Notes |
| --- | --- | --- |
| `id` | integer primary key autoincrement | Local source id |
| `name` | text not null | Display name |
| `kind` | text not null | Connector kind |
| `config_json` | json/text not null | Connector config |
| `enabled` | boolean not null | Source enabled flag |
| `last_check_status` | text nullable | Last connector check result |
| `last_checked_at` | datetime nullable | Last connector check time |
| `created_at` | datetime not null | Row creation time |
| `updated_at` | datetime not null | Row update time |

### `sync_job`

Stores local scheduling configuration. `source_id` is a plain integer reference.

| Column | Type | Notes |
| --- | --- | --- |
| `id` | integer primary key autoincrement | Local job id |
| `source_id` | integer not null | Plain reference to source |
| `name` | text not null | Display name |
| `enabled` | boolean not null | Job enabled flag |
| `schedule_kind` | text not null | `manual` or `interval` |
| `schedule_interval_seconds` | integer nullable | Required for interval jobs |
| `status` | text not null | Runtime job status |
| `cursor` | text nullable | Connector scan cursor |
| `last_run_at` | datetime nullable | Last run timestamp |
| `last_run_status` | text nullable | Last run status snapshot |
| `last_run_id` | integer nullable | Plain reference to last run |
| `created_at` | datetime not null | Row creation time |
| `updated_at` | datetime not null | Row update time |

### `sync_run`

Stores one sync execution. It carries display snapshots so run history remains readable even if source/job names change later.

| Column | Type | Notes |
| --- | --- | --- |
| `id` | integer primary key autoincrement | Local run id |
| `job_id` | integer not null | Plain reference to job |
| `source_id` | integer not null | Plain reference to source |
| `job_name` | text not null | Snapshot at run start |
| `source_name` | text not null | Snapshot at run start |
| `status` | text not null | Run status |
| `started_at` | datetime not null | Start time |
| `finished_at` | datetime nullable | Finish time |
| `processed_count` | integer not null | Items scanned |
| `synced_count` | integer not null | Items written |
| `skipped_count` | integer not null | Items skipped |
| `failed_count` | integer not null | Item failures |
| `deleted_count` | integer not null default 0 | Items marked deleted on source |
| `bytes_written` | integer not null default 0 | Bytes written to vault |
| `created_at` | datetime not null | Row creation time |
| `updated_at` | datetime not null | Row update time |

### `sync_item`

Stores the current index for source items. This table is optimized for sync planning and current item listing, not complete historical replay.

| Column | Type | Notes |
| --- | --- | --- |
| `id` | integer primary key autoincrement | Local item row id |
| `source_id` | integer not null | Plain reference to source |
| `last_run_id` | integer nullable | Last run that produced this item's current outcome |
| `source_path` | text not null | Source-relative path |
| `item_type` | text not null | File, directory, virtual document |
| `status` | text not null | Current item status |
| `size` | integer nullable | Current source size |
| `etag` | text nullable | Current source etag |
| `modified_at` | datetime nullable | Current source modified time |
| `content_hash` | text nullable | Local content hash when available |
| `local_path` | text nullable | Vault path for synced file |
| `metadata_json` | json/text nullable | Reserved connector metadata |
| `last_seen_at` | datetime nullable | Last time the item was observed |
| `synced_at` | datetime nullable | Last successful sync time |
| `deleted_on_source_at` | datetime nullable | Soft deletion timestamp |
| `created_at` | datetime not null | Row creation time |
| `updated_at` | datetime not null | Row update time |

Business identity is `UNIQUE(source_id, source_path)`, not the integer primary key.

### `sync_error`

Stores run and item errors without binding to item rows. `item_id` should be removed unless there is a concrete current use that cannot be served by `source_path`.

| Column | Type | Notes |
| --- | --- | --- |
| `id` | integer primary key autoincrement | Local error id |
| `source_id` | integer nullable | Plain reference to source |
| `job_id` | integer nullable | Plain reference to job |
| `run_id` | integer nullable | Plain reference to run |
| `source_path` | text nullable | Source path involved in the error |
| `error_kind` | text not null | Error category |
| `message` | text not null | Error message |
| `created_at` | datetime not null | Row creation time |

### `app_setting`

Keep the existing key-value shape.

| Column | Type | Notes |
| --- | --- | --- |
| `key` | text primary key | Setting key |
| `value_json` | json/text not null | Setting value |
| `updated_at` | datetime not null | Last update time |

## Indexes

Create indexes explicitly after SeaORM schema sync, using `CREATE INDEX IF NOT EXISTS` and `CREATE UNIQUE INDEX IF NOT EXISTS`.

Required indexes:

```sql
CREATE INDEX IF NOT EXISTS idx_sync_job_source_id
ON sync_job(source_id);

CREATE INDEX IF NOT EXISTS idx_sync_job_enabled_schedule
ON sync_job(enabled, schedule_kind, schedule_interval_seconds);

CREATE INDEX IF NOT EXISTS idx_sync_run_started_at
ON sync_run(started_at DESC);

CREATE INDEX IF NOT EXISTS idx_sync_run_job_id
ON sync_run(job_id);

CREATE INDEX IF NOT EXISTS idx_sync_run_source_id
ON sync_run(source_id);

CREATE UNIQUE INDEX IF NOT EXISTS idx_sync_item_source_path
ON sync_item(source_id, source_path);

CREATE INDEX IF NOT EXISTS idx_sync_item_source_status_path
ON sync_item(source_id, status, source_path);

CREATE INDEX IF NOT EXISTS idx_sync_item_source_last_run
ON sync_item(source_id, last_run_id);

CREATE INDEX IF NOT EXISTS idx_sync_error_run_created
ON sync_error(run_id, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_sync_error_source_created
ON sync_error(source_id, created_at DESC);
```

These indexes match current query paths: list jobs by source, list runs by time, list items by source/status/path, find current item state by source path, mark unseen items after a run, and list errors by run or source.

## Sync Write Flow

At run start:

1. Load the job and source through the repository.
2. Insert `sync_run` with `job_id`, `source_id`, `job_name`, and `source_name`.
3. Set the job status to `running`.

During scan:

1. For each source snapshot, load current state by `(source_id, source_path)`.
2. Plan skip/sync based on current state.
3. Upsert `sync_item` by `(source_id, source_path)`.
4. Set `last_run_id` to the current run for every item outcome.
5. Insert `sync_error` for failed item outcomes using `run_id`, `source_id`, and `source_path`.

After scan:

1. Mark deleted items with a single database update:

   ```sql
   UPDATE sync_item
   SET status = 'deleted_on_source',
       deleted_on_source_at = ?,
       last_run_id = ?,
       updated_at = ?
   WHERE source_id = ?
     AND deleted_on_source_at IS NULL
     AND (last_run_id IS NULL OR last_run_id != ?);
   ```

2. Store the deleted row count on `sync_run.deleted_count` so run history is stable after later item updates.
3. Finish `sync_run` with final counters and `bytes_written`.
4. Reset job status and last-run snapshot fields.

This removes the need to load all known item states into memory only to detect deletions.

## API and CLI Impact

Public IDs change from UUID strings to local integers:

- `sourceId`: integer
- `jobId`: integer
- `runId`: integer
- `itemId`: integer where exposed
- `hoarder sync run --job-id 1` replaces UUID job ids

OpenAPI schemas should use integer identifiers for these local resources.

This is a breaking change for existing local databases and any scripts that pass UUIDs. That is acceptable for the current pre-1.0 local MVP, but the release notes should say it clearly.

## Error Handling

The repository should distinguish three cases:

- Required live references are missing: return `NotFound`. Example: starting a job whose source no longer exists.
- Historical soft references are missing: return the historical row with available snapshots. Example: run detail should still display `source_name` and `job_name`.
- Stored enum-like strings are invalid: return a database error, as today.

Foreign-key absence should not hide logical corruption. It only moves enforcement from SQLite constraints to repository checks where Hoarder can give domain-specific errors.

## Migration Strategy

Because this changes primary key types, relationship shape, and public ID format, treat it as a schema reset unless compatibility becomes a hard requirement.

Recommended implementation path:

1. Add the new entities and repository ID types behind tests.
2. Update schema sync and explicit index creation.
3. Update tests to use integer IDs.
4. Document that existing development databases should be recreated.

Optional compatibility path:

1. Create new tables with integer ids.
2. Copy old rows while building UUID-to-integer maps for source, job, run, and item.
3. Rename old tables to backups.
4. Rename new tables into place.

Do not start with the compatibility path unless there is real user data to preserve. It adds substantial complexity for a pre-1.0 local database.

## Testing Strategy

Backend tests should cover:

- Schema sync creates all tables and explicit indexes.
- Entity definitions no longer generate foreign keys.
- Source/job/run/item/error rows can be inserted without database-level relations.
- Creating a job with a missing source returns `NotFound` or validation error from repository code.
- Starting a run snapshots `source_name` and `job_name`.
- Sync item upsert preserves `UNIQUE(source_id, source_path)`.
- Deleted-source marking happens through a database update keyed by `last_run_id`.
- API and CLI accept integer IDs.
- OpenAPI reports integer IDs.

Performance-focused regression tests should seed many `sync_item` rows and verify that deletion marking does not require loading every item into Rust memory.

## Alternatives Considered

### Keep UUIDs and remove only foreign keys

This reduces relationship coupling but keeps large string/binary identifiers everywhere. It also preserves an API shape that feels heavier than necessary for a local single-database tool.

Rejected because local integer IDs fit Hoarder better and simplify CLI usage.

### Keep a few core foreign keys

This would preserve database-enforced integrity for source/job/run. It also keeps schema migrations, deletion behavior, and SeaORM relationship generation more complex.

Rejected because Hoarder benefits more from soft historical references and simple local schema evolution.

### Collapse all sync tables into one table

This would reduce table count, but it would mix source configuration, scheduling, run history, item state, and error logs into one broad table or JSON blob.

Rejected because it makes queries and tests less clear. The problem is coupling, not table count.

## Consequences

Positive consequences:

- The schema is easier to understand and migrate.
- Local IDs are shorter and easier to use in CLI workflows.
- Sync item planning can rely on targeted indexes.
- Historical run detail is more resilient because it stores display snapshots.
- Removing database foreign keys avoids SeaORM relationship churn.

Trade-offs:

- The database no longer enforces cross-table integrity.
- Repository tests become more important.
- Existing UUID-based local databases need reset or explicit migration.
- API clients and scripts must update to integer IDs.
