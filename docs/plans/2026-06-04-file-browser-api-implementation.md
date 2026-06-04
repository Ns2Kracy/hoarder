# File Browser API Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add `GET /api/files` so the console can browse synced source data by directory while seeing sync metadata for concrete files and directories.

**Architecture:** The API reads from `sync_item` rows and builds a one-level directory listing for a required `sourceId` and optional `path`. It reuses `ItemDto` for concrete item metadata and validates requested paths with the existing vault path normalization rules.

**Tech Stack:** Rust 2024, Axum, SeaORM, SQLite, Serde, existing Hoarder API DTO patterns.

---

### Task 1: Add failing API route tests

**Files:**
- Modify: `tests/api_routes.rs`

**Step 1: Write test helpers**

Add imports for `chrono::Utc`, `hoarder::core::types::{ItemType, SyncStatus}`, `hoarder::sync::repository::ItemSyncOutcome`, and `hoarder::sync::repository::SyncRepository`.

Add a helper near `set_job_running`:

```rust
async fn record_item(repository: &SeaOrmRepository, outcome: ItemSyncOutcome) {
    repository.record_item_outcome(outcome).await.unwrap();
}
```

**Step 2: Write root listing test**

Add:

```rust
#[tokio::test]
async fn api_routes_files_returns_source_root_children() {
    let test = TestApp::new().await;
    let now = Utc::now();
    record_item(&test.repository, item_outcome(test.source_id, "readme.md", ItemType::File, SyncStatus::Synced, now)).await;
    record_item(&test.repository, item_outcome(test.source_id, "docs/guide.md", ItemType::File, SyncStatus::Synced, now)).await;

    let response = request(test.app.clone(), "GET", &format!("/api/files?sourceId={}", test.source_id), None).await;

    assert_eq!(response.status, 200);
    assert_eq!(response.body["sourceId"], json!(test.source_id.as_i64()));
    assert_eq!(response.body["path"], json!(""));
    assert_eq!(response.body["entries"][0]["kind"], json!("directory"));
    assert_eq!(response.body["entries"][0]["name"], json!("docs"));
    assert_eq!(response.body["entries"][0]["item"], Value::Null);
    assert_eq!(response.body["entries"][1]["kind"], json!("file"));
    assert_eq!(response.body["entries"][1]["name"], json!("readme.md"));
    assert_eq!(response.body["entries"][1]["item"]["status"], json!("synced"));
}
```

Add `item_outcome` helper that fills `ItemSyncOutcome` fields with the provided path, type, status, timestamp, and simple metadata.

**Step 3: Write nested and error tests**

Add tests for:

- `GET /api/files?sourceId=<id>&path=docs` returns `docs/guide.md` and inferred `docs/nested` only.
- `GET /api/files?sourceId=<id>&path=../escape` returns `400`.
- `GET /api/files?sourceId=999999` returns `404`.

**Step 4: Run tests and confirm failure**

Run: `cargo test --test api_routes api_routes_files -- --nocapture`

Expected: compile failure or route assertions fail because `/api/files` does not exist yet.

### Task 2: Add API DTOs and route

**Files:**
- Modify: `src/api/types.rs`
- Modify: `src/api/routes.rs`

**Step 1: Add DTOs**

In `src/api/types.rs`, add:

```rust
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileBrowseQuery {
    pub source_id: SourceId,
    pub path: Option<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileBrowseResponse {
    pub source_id: SourceId,
    pub path: String,
    pub entries: Vec<FileEntryDto>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileEntryDto {
    pub name: String,
    pub path: String,
    pub kind: FileEntryKind,
    pub item: Option<ItemDto>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FileEntryKind {
    Directory,
    File,
    VirtualDocument,
}
```

**Step 2: Add route**

In `src/api/routes.rs`, import `FileBrowseQuery` and `FileBrowseResponse`, add `.route("/api/files", get(browse_files))`, and implement:

```rust
async fn browse_files(
    State(state): State<ApiState>,
    Query(query): Query<FileBrowseQuery>,
) -> Result<Json<FileBrowseResponse>, ApiError> {
    Ok(Json(run_service::browse_files(state.repository(), query).await?))
}
```

**Step 3: Run targeted test**

Run: `cargo test --test api_routes api_routes_files -- --nocapture`

Expected: compile failure because `run_service::browse_files` is not implemented.

### Task 3: Implement file browsing service

**Files:**
- Modify: `src/app/run_service.rs`

**Step 1: Validate source and path**

In `browse_files`, load the source by id with `repository.load_source(query.source_id).await?`. Normalize `query.path` as follows:

- `None` or empty/whitespace-only becomes `""`.
- Non-empty path calls `normalize_source_path`.

**Step 2: Query active source items**

Query `sync_item::Entity::find()` filtered by `source_id` and excluding `Status == "deleted_on_source"`, ordered by `SourcePath`.

**Step 3: Build direct children**

For each item:

- If browsing root, use the whole `source_path` as the relative path.
- If browsing a nested path, only consider items equal to the requested path or starting with `requested_path + "/"`; skip the item equal to the directory itself.
- Split the remaining relative path once on `/`.
- No slash means a concrete direct child; create a `FileEntryDto` with `item = Some(item_dto_from_model(item))` and kind mapped from `ItemType`.
- Slash means an inferred directory child; create or keep a `FileEntryDto` with `kind = Directory`, `path = requested_path/name`, and `item = None`.
- If a concrete directory row exists for the same child path, prefer `item = Some(...)` over the inferred entry.

Use `BTreeMap<String, FileEntryDto>` keyed by child path for stable ordering.

**Step 4: Return response**

Return `FileBrowseResponse { source_id, path, entries }` where entries are sorted with directories before files for a predictable browser UI.

**Step 5: Run targeted test**

Run: `cargo test --test api_routes api_routes_files -- --nocapture`

Expected: all file browser tests pass.

### Task 4: Document OpenAPI contract

**Files:**
- Modify: `src/api/openapi.rs`
- Modify: `tests/api_routes.rs`

**Step 1: Add path and schemas**

Add `/api/files` to `paths()`, with operation id `browseFiles`, query parameters `sourceId` and `path`, and response schema `FileBrowseResponse`.

Add schemas for `FileBrowseResponse`, `FileEntryDto`, and `FileEntryKind`. `FileEntryDto.item` should be `oneOf` `ItemDto` or `null`.

**Step 2: Extend OpenAPI test**

Add `/api/files` to `api_routes_openapi_spec_lists_current_routes` and assert `FileBrowseResponse` exists.

**Step 3: Run OpenAPI test**

Run: `cargo test --test api_routes api_routes_openapi_spec_lists_current_routes -- --nocapture`

Expected: pass.

### Task 5: Verify and format

**Files:**
- All modified Rust files

**Step 1: Format**

Run: `cargo fmt --check`

Expected: pass. If it fails, run `cargo fmt`, then rerun `cargo fmt --check`.

**Step 2: Run API route tests**

Run: `cargo test --test api_routes`

Expected: pass.

**Step 3: Run full Rust tests**

Run: `cargo test`

Expected: pass.

**Step 4: Commit**

Commit message:

```bash
git add docs/plans/2026-06-04-file-browser-api-design.md docs/plans/2026-06-04-file-browser-api-implementation.md src/api/types.rs src/api/routes.rs src/api/openapi.rs src/app/run_service.rs tests/api_routes.rs
git commit -m "feat: add file browser api"
```
