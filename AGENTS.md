# Repository Guidelines

## Project Structure & Module Organization

Hoarder is a Rust 2024 workspace with a CLI binary, Axum API, SQLite persistence, OpenDAL connectors, one-way sync, and a Svelte/Vite console.

- `crates/hoarder-cli/` contains the `hoarder` binary and Clap command handlers.
- `crates/hoarder-server/` contains API routes, app services, SeaORM entities/repositories, assets, middleware, config, logging, and server lifecycle.
- `crates/hoarder-sync/` contains the sync engine, planner, repository trait, cancellation, and vault writer.
- `crates/hoarder-connectors/` contains connector traits and adapters.
- `crates/hoarder-core/` contains stable domain types, errors, and vault path helpers.
- Integration tests live under each crate's `tests/` directory.
- `web/src/` contains Svelte 5 routes, components, API helpers, and shared types. `web/dist/` is generated and embedded by `crates/hoarder-server/src/assets.rs`.
- `docs/` stores development notes and implementation plans.

## Build, Test, and Development Commands

- `cd web && bun install`: install frontend dependencies.
- `cd web && bun run dev`: start the Vite dev server on loopback.
- `cd web && bun run build`: build `web/dist` for embedding in the Rust binary.
- `cargo run -p hoarder-cli -- serve`: run the local API and embedded console at `127.0.0.1:4761`.
- `cargo run -p hoarder-cli -- db sync`: synchronize the SQLite schema from SeaORM entities.
- `cargo build -p hoarder-cli --release`: build the packaged binary. Rebuild the frontend first when `web/src/` changes.
- `cargo fmt --check`, `cargo clippy --workspace --all-targets --all-features`, `cargo test --workspace`, and `cd web && bun run verify`: pre-merge checks.

## Coding Style & Naming Conventions

Rust warnings and strict Clippy lint groups are denied through workspace lints in `Cargo.toml`; keep code formatted with `cargo fmt`. Use Rust module names in `snake_case`, types in `PascalCase`, and functions/variables in `snake_case`. Keep connector-specific logic behind `crates/hoarder-connectors/src/traits.rs`. For Svelte, use TypeScript, Svelte 5 conventions, and `PascalCase` component names.

## Testing Guidelines

Add or update integration tests under the affected crate's `tests/` directory. Name test files by feature area and test functions by expected behavior. Run `cargo test --workspace` for Rust changes and `bun run verify` inside `web/` for UI changes. For packaging changes, run `bun run build` before `cargo build -p hoarder-cli --release`.

## Commit & Pull Request Guidelines

Recent history uses short Conventional Commit-style subjects such as `feat: add packaging integration`, `refactor: use uuid v4 identifiers`, and `docs: expand project readme`. Prefer `feat:`, `fix:`, `refactor:`, `docs:`, `test:`, or `chore:`. Pull requests should summarize changes, list verification commands, link related issues or plans, and include screenshots for visible UI changes.

## Security & Configuration Tips

Default local serving binds to `127.0.0.1`. Do not commit real vault data, generated databases such as `hoarder.db`, secrets, or connector credentials. Use `--config ./hoarder.config.json` for local path and concurrency overrides.
