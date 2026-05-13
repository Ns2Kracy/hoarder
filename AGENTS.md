# Repository Guidelines

## Project Structure & Module Organization

Hoarder is a Rust 2024 binary with an Axum API, SQLite persistence, OpenDAL connectors, one-way sync, and a Svelte/Vite console.

- `src/main.rs`, `src/cli.rs`, and `src/server.rs` contain entry points.
- `src/api/`, `src/db/`, `src/entity/`, `src/connectors/`, `src/sync/`, and `src/core/` hold Rust modules.
- `tests/` contains integration tests such as `sync_engine.rs`, `api_routes.rs`, and `vault_path.rs`.
- `web/src/` contains Svelte 5 routes, components, API helpers, and shared types. `web/dist/` is generated and embedded by `src/assets.rs`.
- `docs/` stores development notes and implementation plans.

## Build, Test, and Development Commands

- `cd web && bun install`: install frontend dependencies.
- `cd web && bun run dev`: start the Vite dev server on loopback.
- `cd web && bun run build`: build `web/dist` for embedding in the Rust binary.
- `cargo run -- serve`: run the local API and embedded console at `127.0.0.1:4761`.
- `cargo run -- db sync`: synchronize the SQLite schema from SeaORM entities.
- `cargo build --release`: build the packaged binary. Rebuild the frontend first when `web/src/` changes.
- `cargo fmt --check`, `cargo clippy --all-targets --all-features`, `cargo test`, and `cd web && bun run verify`: pre-merge checks.

## Coding Style & Naming Conventions

Rust warnings and strict Clippy lint groups are denied in `Cargo.toml`; keep code formatted with `cargo fmt`. Use Rust module names in `snake_case`, types in `PascalCase`, and functions/variables in `snake_case`. Keep connector-specific logic behind `src/connectors/traits.rs`. For Svelte, use TypeScript, Svelte 5 conventions, and `PascalCase` component names.

## Testing Guidelines

Add or update integration tests in `tests/` for backend behavior changes. Name test files by feature area and test functions by expected behavior. Run `cargo test` for Rust changes and `bun run verify` inside `web/` for UI changes. For packaging changes, run `bun run build` before `cargo build --release`.

## Commit & Pull Request Guidelines

Recent history uses short Conventional Commit-style subjects such as `feat: add packaging integration`, `refactor: use uuid v4 identifiers`, and `docs: expand project readme`. Prefer `feat:`, `fix:`, `refactor:`, `docs:`, `test:`, or `chore:`. Pull requests should summarize changes, list verification commands, link related issues or plans, and include screenshots for visible UI changes.

## Security & Configuration Tips

Default local serving binds to `127.0.0.1`. Do not commit real vault data, generated databases such as `hoarder.db`, secrets, or connector credentials. Use `--config ./hoarder.config.json` for local path and concurrency overrides.
