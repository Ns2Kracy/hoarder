# File Browser API Design

## Goal

Add a file browsing API that lets the console browse synced source data as a directory tree while preserving sync metadata for each concrete item.

## Context

Hoarder already exposes `GET /api/items` as a flat operational list of `sync_item` records. That endpoint is useful for filtering by source, run, and status, but it is not shaped like a file browser. The database already stores the fields needed for browsing: `source_id`, `source_path`, `item_type`, `status`, `size`, `etag`, `modified_at`, `content_hash`, and `metadata_json`.

The sync writer also normalizes vault paths through `core::vault_path`, so the browser API should reuse the same path validation rules instead of accepting arbitrary path strings.

## API Contract

Add:

```http
GET /api/files?sourceId=1&path=docs
```

Query parameters:

- `sourceId` is required and selects the source namespace.
- `path` is optional. Missing or empty means the source root.

Response:

```json
{
  "sourceId": 1,
  "path": "docs",
  "entries": [
    {
      "name": "readme.md",
      "path": "docs/readme.md",
      "kind": "file",
      "item": {
        "id": 42,
        "sourceId": 1,
        "sourcePath": "docs/readme.md",
        "itemType": "file",
        "status": "synced",
        "size": 8452,
        "modifiedAt": "2026-05-12T01:24:00Z",
        "contentHash": "sha256:..."
      }
    },
    {
      "name": "nested",
      "path": "docs/nested",
      "kind": "directory",
      "item": null
    }
  ]
}
```

## Behavior

- Return only direct children of the requested directory.
- Build directory entries from `sync_item.source_path` prefixes.
- Attach `ItemDto` for concrete records in the requested directory.
- Use `item = null` for inferred directories that do not have their own `sync_item` row.
- Reject invalid paths such as `..`, absolute paths, and `.hoarder` using existing vault path normalization.
- Return `404` when the source does not exist.
- Exclude `deleted_on_source` items for the initial implementation. The API can grow an `includeDeleted` query parameter later if the console needs it.

## Architecture

- Add DTOs and query type in `src/api/types.rs`.
- Add `GET /api/files` in `src/api/routes.rs`.
- Implement browsing in `src/app/run_service.rs` because it already owns sync item read models.
- Document the route and schemas in `src/api/openapi.rs`.
- Add integration tests in `tests/api_routes.rs`.

## Testing

- Root browsing returns direct files and inferred directories for a source.
- Nested browsing returns only the requested directory's direct children.
- Invalid paths return `400`.
- Unknown sources return `404`.
