# Workspace Crate Split Design

Date: 2026-06-05
Status: Approved design

## Objective

Split Hoarder from one Rust package into a small Cargo workspace that makes the sync runtime, connector adapters, core domain, server control plane, and CLI entrypoint easier to reason about independently.

The split should preserve the current user-facing binary name, API behavior, database behavior, and test coverage while making dependency direction enforce architecture rules at compile time.

## Target Workspace

```text
crates/
  hoarder-core/
  hoarder-connectors/
  hoarder-sync/
  hoarder-server/
  hoarder-cli/
Cargo.toml
```

The binary remains named `hoarder`, even though the package that owns it becomes `hoarder-cli`.

## Dependency Direction

```text
hoarder-cli
  -> hoarder-server

hoarder-server
  -> hoarder-sync
  -> hoarder-connectors
  -> hoarder-core

hoarder-sync
  -> hoarder-connectors
  -> hoarder-core

hoarder-connectors
  -> hoarder-core

hoarder-core
  -> external crates only
```

No lower-level crate may depend on `hoarder-server` or `hoarder-cli`.

## Crate Responsibilities

### `hoarder-core`

Owns stable domain types and cross-crate error semantics.

Initial files:

```text
src/core/types.rs
src/core/vault_path.rs
src/error.rs
```

Expected public interface:

```rust
pub mod types;
pub mod vault_path;

pub use error::{AppError, AppResult};
```

This crate must not depend on Axum, SeaORM, OpenDAL, `utoipa`, or server configuration.

### `hoarder-connectors`

Owns the connector contract and all concrete connector adapters.

Initial files:

```text
src/connectors/traits.rs
src/connectors/registry.rs
src/connectors/opendal/**
src/connectors/notion.rs
src/connectors/feishu.rs
src/connectors/plugin.rs
```

It depends on `hoarder-core`, OpenDAL, Reqwest, and connector-specific crates. It does not depend on sync, server, API, database, or CLI code.

### `hoarder-sync`

Owns the sync runtime interface and implementation.

Initial files:

```text
src/sync/engine.rs
src/sync/planner.rs
src/sync/repository.rs
src/sync/vault_writer.rs
src/app/run_control.rs
```

`src/app/run_control.rs` moves here because cancellation is part of sync runtime behavior, not server orchestration. This removes the current reverse dependency from sync to app.

This crate depends on `hoarder-core` and `hoarder-connectors`, but not on `hoarder-server`.

### `hoarder-server`

Owns the local control plane and composition root.

Initial files:

```text
src/api/**
src/app/**
src/assets.rs
src/config.rs
src/db/**
src/entity/**
src/logging.rs
src/middleware/**
src/server.rs
```

This crate intentionally remains broad in the first split. It keeps Axum, SeaORM, embedded frontend assets, scheduler wiring, OpenAPI generation, and app services together so the workspace migration does not also become an app/store/API decomposition.

### `hoarder-cli`

Owns command parsing and the executable entrypoint.

Initial files:

```text
src/main.rs
src/cli.rs
```

Its `Cargo.toml` declares:

```toml
[[bin]]
name = "hoarder"
path = "src/main.rs"
```

## Known Dependency Corrections

Before or during extraction, fix these dependency direction issues:

1. `hoarder-sync` must not depend on `app::run_control`.
   Move `CancellationToken` and related sync cancellation primitives into `hoarder-sync`.

2. `hoarder-core` must not know about API documentation.
   Keep `utoipa` schema-only wrappers in `hoarder-server::api`, as in the current OpenAPI implementation.

3. `hoarder-connectors` must not depend on sync runtime.
   The connector contract should only return core domain snapshots and byte streams.

4. `hoarder-server::app` currently uses API DTOs directly.
   This can stay inside `hoarder-server` for the first split, but it should be revisited after the workspace migration if app services need to become their own crate.

## Migration Strategy

Use a small-commit sequence. Each commit should leave the workspace building and tests passing.

1. Convert root `Cargo.toml` to a workspace and add empty member packages.
2. Extract `hoarder-core` and update imports.
3. Extract `hoarder-connectors` and update imports.
4. Move cancellation into `hoarder-sync`, then extract `hoarder-sync`.
5. Move remaining control-plane code into `hoarder-server`.
6. Move CLI files into `hoarder-cli` while preserving the `hoarder` binary name.

## Non-Goals

- Do not split `hoarder-app`, `hoarder-store`, or `hoarder-api` in this migration.
- Do not change HTTP routes, OpenAPI output, database schema, or CLI command behavior.
- Do not move frontend files into the Rust workspace.
- Do not rewrite app services or repository interfaces beyond what is required for dependency direction.

## Verification

Every migration commit should run:

```bash
cargo fmt --check
cargo clippy --workspace --all-targets --all-features
cargo test --workspace
```

The existing frontend verification is not required unless Rust asset embedding or generated frontend artifacts change.
