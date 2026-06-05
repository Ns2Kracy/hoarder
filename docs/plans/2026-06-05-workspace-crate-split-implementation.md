# Workspace Crate Split Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Split Hoarder into `hoarder-core`, `hoarder-connectors`, `hoarder-sync`, `hoarder-server`, and `hoarder-cli` workspace crates without changing runtime behavior.

**Architecture:** Move from one library/binary package to a Cargo workspace with dependency direction enforced by package boundaries. Keep `hoarder-server` as the broad composition crate for API, app services, database, assets, and server lifecycle; move stable domain, connector adapters, sync runtime, and CLI entrypoint into focused crates.

**Tech Stack:** Rust 2024, Cargo workspace, Axum, SeaORM, OpenDAL, Tokio, `utoipa`, Svelte embedded assets.

---

## Ground Rules

- Keep the executable command named `hoarder`.
- Do not change HTTP routes, OpenAPI output, database schema, CLI command names, or sync behavior.
- Prefer move-only commits plus import rewrites. Avoid refactors that are not needed for crate extraction.
- Keep `utoipa` and Scalar dependencies inside `hoarder-server`.
- Keep SeaORM and Axum inside `hoarder-server` for this migration.
- Use shim modules only temporarily when they reduce churn, and remove them by the final task.

## Final Dependency Graph

```text
hoarder-cli -> hoarder-server
hoarder-server -> hoarder-sync, hoarder-connectors, hoarder-core
hoarder-sync -> hoarder-connectors, hoarder-core
hoarder-connectors -> hoarder-core
hoarder-core -> external crates only
```

## Task 1: Create Workspace Skeleton

**Files:**
- Modify: `Cargo.toml`
- Create: `crates/hoarder-core/Cargo.toml`
- Create: `crates/hoarder-core/src/lib.rs`
- Create: `crates/hoarder-connectors/Cargo.toml`
- Create: `crates/hoarder-connectors/src/lib.rs`
- Create: `crates/hoarder-sync/Cargo.toml`
- Create: `crates/hoarder-sync/src/lib.rs`
- Create: `crates/hoarder-server/Cargo.toml`
- Create: `crates/hoarder-server/src/lib.rs`
- Create: `crates/hoarder-cli/Cargo.toml`
- Create: `crates/hoarder-cli/src/main.rs`

**Step 1: Add workspace metadata while keeping the current root package temporarily**

Update root `Cargo.toml` to keep the existing `[package]` and add this workspace block near the top:

```toml
[workspace]
members = [
  ".",
  "crates/hoarder-core",
  "crates/hoarder-connectors",
  "crates/hoarder-sync",
  "crates/hoarder-server",
  "crates/hoarder-cli",
]
resolver = "3"

[workspace.package]
edition = "2024"
license = "MIT"
repository = "https://github.com/ns2kracy/hoarder"
rust-version = "1.85"
```

Do not move existing dependencies yet. This keeps the current package building while empty crates are introduced.

**Step 2: Create placeholder member crates**

Use minimal package files. Example for `crates/hoarder-core/Cargo.toml`:

```toml
[package]
name = "hoarder-core"
version = "0.1.0"
edition.workspace = true
license.workspace = true
repository.workspace = true
rust-version.workspace = true

[dependencies]
```

Use equivalent package stubs for `hoarder-connectors`, `hoarder-sync`, `hoarder-server`, and `hoarder-cli`.

For each new library crate, create `src/lib.rs` with:

```rust
#![deny(warnings)]
```

For `crates/hoarder-cli/src/main.rs`, create:

```rust
fn main() {}
```

This binary is temporary and will be replaced in Task 6.

**Step 3: Run workspace metadata check**

Run:

```bash
cargo metadata --format-version 1 >/dev/null
```

Expected: exit 0.

**Step 4: Run full checks**

Run:

```bash
cargo fmt --check
cargo clippy --workspace --all-targets --all-features
cargo test --workspace
```

Expected: all pass. Existing root package tests still run.

**Step 5: Commit**

```bash
git add Cargo.toml crates
git commit -m "chore: add cargo workspace skeleton"
```

## Task 2: Extract `hoarder-core`

**Files:**
- Move: `src/core/types.rs` -> `crates/hoarder-core/src/types.rs`
- Move: `src/core/vault_path.rs` -> `crates/hoarder-core/src/vault_path.rs`
- Move: `src/error.rs` -> `crates/hoarder-core/src/error.rs`
- Modify: `crates/hoarder-core/src/lib.rs`
- Modify: `crates/hoarder-core/Cargo.toml`
- Modify: root `Cargo.toml`
- Replace: `src/core/mod.rs`
- Replace: `src/error.rs`
- Modify imports in current root package only when needed

**Step 1: Add `hoarder-core` dependencies**

In `crates/hoarder-core/Cargo.toml`, add only dependencies needed by moved files:

```toml
[dependencies]
chrono = { version = "0.4.44", features = ["serde"] }
serde = { version = "1.0.228", features = ["derive"] }
serde_json = "1.0.149"
thiserror = "2.0.18"
```

**Step 2: Move files and define core library interface**

Create `crates/hoarder-core/src/lib.rs`:

```rust
pub mod error;
pub mod types;
pub mod vault_path;

pub use error::{AppError, AppResult};
```

Update moved files:

- In `crates/hoarder-core/src/vault_path.rs`, replace `crate::{AppError, AppResult}` with `crate::{AppError, AppResult}` if the root re-export still resolves after move.
- In `crates/hoarder-core/src/types.rs`, keep imports local to external crates only.
- In `crates/hoarder-core/src/error.rs`, remove references to old root modules if any appear.

**Step 3: Add root package dependency and compatibility shims**

In root `Cargo.toml`, add:

```toml
hoarder-core = { path = "crates/hoarder-core" }
```

Replace `src/core/mod.rs` with:

```rust
pub use hoarder_core::{types, vault_path};
```

Recreate `src/error.rs` as:

```rust
pub use hoarder_core::error::*;
```

Keep `src/lib.rs` exports unchanged for now:

```rust
pub mod core;
pub mod error;
pub use error::{AppError, AppResult};
```

This preserves existing imports such as `hoarder::core::types::SourceId` during the migration.

**Step 4: Run focused tests**

Run:

```bash
cargo test -p hoarder-core
cargo test vault_path core_types
```

Expected: all pass.

**Step 5: Run workspace checks**

Run:

```bash
cargo fmt --check
cargo clippy --workspace --all-targets --all-features
cargo test --workspace
```

Expected: all pass.

**Step 6: Commit**

```bash
git add Cargo.toml crates/hoarder-core src/core src/error.rs src/lib.rs tests
git commit -m "refactor: extract hoarder core crate"
```

## Task 3: Extract `hoarder-connectors`

**Files:**
- Move: `src/connectors/**` -> `crates/hoarder-connectors/src/`
- Modify: `crates/hoarder-connectors/src/lib.rs`
- Modify: `crates/hoarder-connectors/Cargo.toml`
- Modify: root `Cargo.toml`
- Replace: `src/connectors/mod.rs`
- Update imports in moved connector files

**Step 1: Add connector crate dependencies**

In `crates/hoarder-connectors/Cargo.toml`, add dependencies used by connector files:

```toml
[dependencies]
bytes = "1.11.1"
chrono = { version = "0.4.44", features = ["serde"] }
futures = "0.3.32"
hex = "0.4.3"
hoarder-core = { path = "../hoarder-core" }
opendal = { version = "0.56.0", default-features = false, features = [
  "executors-tokio",
  "reqwest-rustls-tls",
  "services-fs",
  "services-s3",
  "services-sftp",
  "services-webdav",
] }
reqwest = { version = "0.12", default-features = false, features = ["json", "rustls-tls"] }
serde = { version = "1.0.228", features = ["derive"] }
serde_json = "1.0.149"
sha2 = "0.11.0"
tokio = { version = "1.52.3", features = ["fs", "io-util", "macros", "rt-multi-thread"] }
```

Trim this list if `cargo check -p hoarder-connectors` proves a dependency is unused.

**Step 2: Move connector files**

Move all files under `src/connectors/` into `crates/hoarder-connectors/src/` preserving subdirectories.

Set `crates/hoarder-connectors/src/lib.rs` to the former module list:

```rust
pub mod feishu;
pub mod notion;
pub mod opendal;
pub mod plugin;
pub mod registry;
pub mod traits;
```

**Step 3: Rewrite connector imports**

In moved files, replace:

```rust
use crate::{core::types::..., error::..., AppResult};
```

with:

```rust
use hoarder_core::{types::..., AppError, AppResult};
```

Replace any `crate::connectors::...` self-references with `crate::...` inside the connector crate.

**Step 4: Add root compatibility shim**

In root `Cargo.toml`, add:

```toml
hoarder-connectors = { path = "crates/hoarder-connectors" }
```

Replace `src/connectors/mod.rs` with:

```rust
pub use hoarder_connectors::*;
```

This keeps existing root package imports compiling during later tasks.

**Step 5: Move connector-focused tests if useful**

Move these tests to `crates/hoarder-connectors/tests/` if they only use connector APIs:

```text
tests/connector_contract.rs
tests/feishu_connector.rs
tests/notion_connector.rs
tests/opendal_config.rs
tests/opendal_fs_connector.rs
tests/opendal_operator.rs
tests/plugin_abi.rs
```

Update imports from:

```rust
use hoarder::connectors::...
use hoarder::core::...
```

to:

```rust
use hoarder_connectors::...
use hoarder_core::...
```

If a test still needs server database/app helpers, leave it under root for now.

**Step 6: Run focused tests**

Run:

```bash
cargo test -p hoarder-connectors
```

Expected: all connector tests pass.

**Step 7: Run workspace checks**

Run:

```bash
cargo fmt --check
cargo clippy --workspace --all-targets --all-features
cargo test --workspace
```

Expected: all pass.

**Step 8: Commit**

```bash
git add Cargo.toml crates/hoarder-connectors src/connectors tests
git commit -m "refactor: extract connector crate"
```

## Task 4: Extract `hoarder-sync`

**Files:**
- Move: `src/app/run_control.rs` -> `crates/hoarder-sync/src/run_control.rs`
- Move: `src/sync/**` -> `crates/hoarder-sync/src/`
- Modify: `crates/hoarder-sync/src/lib.rs`
- Modify: `crates/hoarder-sync/Cargo.toml`
- Modify: root `Cargo.toml`
- Replace: `src/sync/mod.rs`
- Remove or replace: `src/app/run_control.rs`
- Update imports in server/app code

**Step 1: Add sync crate dependencies**

In `crates/hoarder-sync/Cargo.toml`, add:

```toml
[dependencies]
chrono = { version = "0.4.44", features = ["serde"] }
futures = "0.3.32"
hoarder-connectors = { path = "../hoarder-connectors" }
hoarder-core = { path = "../hoarder-core" }
serde_json = "1.0.149"
sha2 = "0.11.0"
tokio = { version = "1.52.3", features = ["fs", "io-util", "macros", "rt-multi-thread"] }
```

**Step 2: Move cancellation into sync crate**

Move `src/app/run_control.rs` to `crates/hoarder-sync/src/run_control.rs`.

Expose it from `crates/hoarder-sync/src/lib.rs`:

```rust
pub mod engine;
pub mod planner;
pub mod repository;
pub mod run_control;
pub mod vault_writer;
```

**Step 3: Move sync files and rewrite imports**

Move `src/sync/engine.rs`, `planner.rs`, `repository.rs`, and `vault_writer.rs` into `crates/hoarder-sync/src/`.

In moved sync files, replace root imports:

```rust
use crate::{AppError, AppResult};
use crate::connectors::traits::{ConnectorConfig, SourceConnector};
use crate::core::types::{...};
```

with crate imports:

```rust
use hoarder_core::{AppError, AppResult};
use hoarder_core::types::{...};
use hoarder_connectors::traits::{ConnectorConfig, SourceConnector};
```

In `engine.rs`, replace:

```rust
use crate::app::run_control::CancellationToken;
```

with:

```rust
use crate::run_control::CancellationToken;
```

**Step 4: Add root compatibility shim**

In root `Cargo.toml`, add:

```toml
hoarder-sync = { path = "crates/hoarder-sync" }
```

Replace `src/sync/mod.rs` with:

```rust
pub use hoarder_sync::*;
```

Replace `src/app/run_control.rs` with:

```rust
pub use hoarder_sync::run_control::*;
```

This keeps server code compiling until Task 5 updates imports directly.

**Step 5: Move sync-focused tests**

Move these tests to `crates/hoarder-sync/tests/` if they do not require server database helpers:

```text
tests/sync_engine.rs
tests/sync_planner.rs
tests/vault_path.rs
tests/vault_writer.rs
```

Update imports from `hoarder::sync` to `hoarder_sync`, from `hoarder::core` to `hoarder_core`, and from `hoarder::connectors` to `hoarder_connectors`.

**Step 6: Run focused tests**

Run:

```bash
cargo test -p hoarder-sync
```

Expected: sync, planner, vault writer, and cancellation tests pass.

**Step 7: Run workspace checks**

Run:

```bash
cargo fmt --check
cargo clippy --workspace --all-targets --all-features
cargo test --workspace
```

Expected: all pass.

**Step 8: Commit**

```bash
git add Cargo.toml crates/hoarder-sync src/sync src/app/run_control.rs tests
git commit -m "refactor: extract sync runtime crate"
```

## Task 5: Split `hoarder-server`

**Files:**
- Move: `src/api/**` -> `crates/hoarder-server/src/api/`
- Move: `src/app/**` -> `crates/hoarder-server/src/app/`
- Move: `src/assets.rs` -> `crates/hoarder-server/src/assets.rs`
- Move: `src/config.rs` -> `crates/hoarder-server/src/config.rs`
- Move: `src/db/**` -> `crates/hoarder-server/src/db/`
- Move: `src/entity/**` -> `crates/hoarder-server/src/entity/`
- Move: `src/logging.rs` -> `crates/hoarder-server/src/logging.rs`
- Move: `src/middleware/**` -> `crates/hoarder-server/src/middleware/`
- Move: `src/server.rs` -> `crates/hoarder-server/src/server.rs`
- Modify: `crates/hoarder-server/src/lib.rs`
- Modify: `crates/hoarder-server/Cargo.toml`
- Move server/app/db/API tests to `crates/hoarder-server/tests/`

**Step 1: Add server crate dependencies**

Move all dependencies not needed by core/connectors/sync/CLI from root `Cargo.toml` into `crates/hoarder-server/Cargo.toml`.

It should include at least:

```toml
[dependencies]
axum = "0.8.9"
chrono = { version = "0.4.44", features = ["serde"] }
futures = "0.3.32"
hoarder-connectors = { path = "../hoarder-connectors" }
hoarder-core = { path = "../hoarder-core" }
hoarder-sync = { path = "../hoarder-sync" }
reqwest = { version = "0.12", default-features = false, features = ["json", "rustls-tls"] }
rust-embed = "8.11.0"
sea-orm = { version = "2.0.0-rc.38", default-features = false, features = [
  "entity-registry",
  "macros",
  "runtime-tokio-rustls",
  "schema-sync",
  "sqlx-sqlite",
  "with-chrono",
  "with-json",
  "with-uuid",
] }
serde = { version = "1.0.228", features = ["derive"] }
serde_json = "1.0.149"
tokio = { version = "1.52.3", features = ["fs", "io-util", "macros", "rt-multi-thread", "signal"] }
tower-http = { version = "0.6.10", features = ["compression-gzip", "cors", "propagate-header", "request-id"] }
tracing = "0.1.44"
tracing-subscriber = { version = "0.3.23", features = ["env-filter"] }
utoipa = { version = "5.5.0", features = ["chrono", "preserve_path_order"] }
utoipa-scalar = { version = "0.3.0", features = ["axum"] }
uuid = { version = "1.23.1", features = ["serde", "v4"] }
```

**Step 2: Move server files**

Move the listed files into `crates/hoarder-server/src/` preserving module layout.

Set `crates/hoarder-server/src/lib.rs` to:

```rust
#![recursion_limit = "4096"]

pub mod api;
pub mod app;
pub mod assets;
pub mod config;
pub mod db;
pub mod entity;
pub mod logging;
pub mod middleware;
pub mod server;

pub use config::AppConfig;
pub use hoarder_core::{AppError, AppResult};
```

**Step 3: Rewrite imports**

Inside moved server files:

- Replace `crate::core::...` with `hoarder_core::...`.
- Replace `crate::connectors::...` with `hoarder_connectors::...`.
- Replace `crate::sync::...` with `hoarder_sync::...`.
- Replace `crate::{AppError, AppResult}` with `hoarder_core::{AppError, AppResult}` unless the file intentionally uses the server crate re-export.
- Replace `crate::app::run_control` with `hoarder_sync::run_control`.

Keep intra-server imports as `crate::api`, `crate::app`, `crate::db`, `crate::entity`, and `crate::server`.

**Step 4: Move server tests**

Move tests that require API, app, DB, server, or embedded assets into `crates/hoarder-server/tests/`:

```text
tests/api_error.rs
tests/api_routes.rs
tests/app_services.rs
tests/db_schema.rs
tests/e2e_local_fs_sync.rs
tests/scheduler.rs
tests/server_cors.rs
tests/settings_repository.rs
tests/static_assets.rs
tests/cli_commands.rs if it still depends on server internals
```

Update imports:

```rust
use hoarder::{...};
```

to:

```rust
use hoarder_server::{...};
use hoarder_core::{...};
use hoarder_connectors::{...};
use hoarder_sync::{...};
```

**Step 5: Run focused server tests**

Run:

```bash
cargo test -p hoarder-server
```

Expected: API, DB, scheduler, CORS, static assets, and e2e local sync tests pass.

**Step 6: Run workspace checks**

Run:

```bash
cargo fmt --check
cargo clippy --workspace --all-targets --all-features
cargo test --workspace
```

Expected: all pass.

**Step 7: Commit**

```bash
git add Cargo.toml crates/hoarder-server src tests
git commit -m "refactor: extract server crate"
```

## Task 6: Split `hoarder-cli` and Make Root Virtual

**Files:**
- Move: `src/main.rs` -> `crates/hoarder-cli/src/main.rs`
- Move: `src/cli.rs` -> `crates/hoarder-cli/src/cli.rs`
- Modify: `crates/hoarder-cli/Cargo.toml`
- Modify: `crates/hoarder-cli/src/main.rs`
- Modify: `crates/hoarder-cli/src/cli.rs`
- Modify: root `Cargo.toml`
- Delete: remaining root `src/` shims once unused
- Move CLI tests to `crates/hoarder-cli/tests/`

**Step 1: Add CLI dependencies**

In `crates/hoarder-cli/Cargo.toml`, configure the package and binary:

```toml
[package]
name = "hoarder-cli"
version = "0.1.0"
edition.workspace = true
license.workspace = true
repository.workspace = true
rust-version.workspace = true

[[bin]]
name = "hoarder"
path = "src/main.rs"

[dependencies]
clap = { version = "4.6.1", features = ["derive", "env"] }
hoarder-core = { path = "../hoarder-core" }
hoarder-server = { path = "../hoarder-server" }
tokio = { version = "1.52.3", features = ["macros", "rt-multi-thread"] }
```

**Step 2: Move CLI files and rewrite imports**

Move `src/main.rs` and `src/cli.rs` into the CLI crate.

In `crates/hoarder-cli/src/main.rs`, imports should point at local `cli` and server/core crates:

```rust
mod cli;

use hoarder_core::AppResult;
```

In `crates/hoarder-cli/src/cli.rs`, replace references like:

```rust
use crate::server;
use crate::config;
```

with:

```rust
use hoarder_server::{config, server};
```

**Step 3: Move CLI tests**

Move CLI-specific tests into `crates/hoarder-cli/tests/`:

```text
tests/cli_parse.rs
tests/cli_commands.rs if not already moved to server
```

Update imports to use `hoarder_cli` only if a library target is added. If no CLI library is needed, keep parse helpers private and test the binary-facing behavior through command construction helpers exposed from `cli.rs` via a small `lib.rs`.

If testing `cli.rs` requires library access, add `crates/hoarder-cli/src/lib.rs`:

```rust
pub mod cli;
```

and update `main.rs` to use `hoarder_cli::cli` instead of `mod cli;`.

**Step 4: Make root a virtual workspace**

After all source and tests have moved out of root, remove root `[package]`, root `[dependencies]`, and root lint tables only after copying lints to workspace-level package crates or each crate as needed.

Root `Cargo.toml` should become:

```toml
[workspace]
members = [
  "crates/hoarder-core",
  "crates/hoarder-connectors",
  "crates/hoarder-sync",
  "crates/hoarder-server",
  "crates/hoarder-cli",
]
resolver = "3"

[workspace.package]
edition = "2024"
license = "MIT"
repository = "https://github.com/ns2kracy/hoarder"
rust-version = "1.85"
```

Preserve strict lints by adding matching `[lints.clippy]` and `[lints.rust]` tables to each crate or by using workspace lints if this repository's toolchain supports them cleanly.

**Step 5: Verify binary still exists**

Run:

```bash
cargo run -p hoarder-cli -- --help
```

Expected: command help prints for binary `hoarder`.

Run:

```bash
cargo build -p hoarder-cli
```

Expected: `target/debug/hoarder` exists.

**Step 6: Run workspace checks**

Run:

```bash
cargo fmt --check
cargo clippy --workspace --all-targets --all-features
cargo test --workspace
```

Expected: all pass.

**Step 7: Commit**

```bash
git add Cargo.toml crates src tests
git commit -m "refactor: split cli crate"
```

## Task 7: Cleanup Docs and CI Commands

**Files:**
- Modify: `AGENTS.md`
- Modify: `README.md` if command examples mention package layout
- Modify: `docs/architecture.md`
- Modify: `docs/development.md`
- Modify: any CI config if present

**Step 1: Update development commands**

Replace single-package Rust checks with workspace checks:

```bash
cargo fmt --check
cargo clippy --workspace --all-targets --all-features
cargo test --workspace
cargo run -p hoarder-cli -- serve
cargo run -p hoarder-cli -- db sync
cargo build -p hoarder-cli --release
```

**Step 2: Update architecture module map**

Update `docs/architecture.md` so the module table points at `crates/hoarder-*` instead of root `src/` paths.

**Step 3: Run doc-sensitive smoke checks**

Run:

```bash
cargo run -p hoarder-cli -- --help
cargo run -p hoarder-cli -- db --help
```

Expected: help output still works.

**Step 4: Run final verification**

Run:

```bash
cargo fmt --check
cargo clippy --workspace --all-targets --all-features
cargo test --workspace
```

Expected: all pass.

**Step 5: Commit**

```bash
git add AGENTS.md README.md docs Cargo.toml crates
git commit -m "docs: update workspace development guide"
```

## Final Acceptance Criteria

- `cargo metadata --format-version 1` reports five workspace member packages and no root package.
- `cargo build -p hoarder-cli` produces `target/debug/hoarder`.
- `cargo run -p hoarder-cli -- --help` works.
- `cargo fmt --check` passes.
- `cargo clippy --workspace --all-targets --all-features` passes.
- `cargo test --workspace` passes.
- `hoarder-core` has no dependencies on Axum, SeaORM, OpenDAL, `utoipa`, or server code.
- `hoarder-connectors` has no dependencies on sync, server, API, DB, or CLI code.
- `hoarder-sync` has no dependencies on server, API, DB, or CLI code.
- `hoarder-server` contains OpenAPI, API, app services, DB/entity, scheduler, static assets, and server lifecycle.
- `hoarder-cli` owns the executable entrypoint and command parsing.
