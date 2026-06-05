# Release And Distribution

Hoarder ships as a single Rust binary with the Svelte console embedded from `web/dist`.

## CI

`.github/workflows/ci.yml` runs on pushes to `main` and pull requests targeting `main`.

The workflow has three gates:

- Rust: `cargo fmt --check`, build embedded `web/dist` assets for `RustEmbed`, `cargo clippy --workspace --all-targets --all-features --message-format=short`, and `cargo test --workspace`.
- Web: `bun install --frozen-lockfile`, `bun run fmt:check`, `bun run lint`, `bun run check`, `bun test`, and `bun run build`.
- Package smoke: build `web/dist`, run `cargo build -p hoarder-cli --release`, then execute `hoarder --help` and `hoarder source templates`.

## Release Artifacts

`.github/workflows/release.yml` runs on `v*` tags and can also be started manually from GitHub Actions.

It builds native artifacts for:

- `hoarder-linux-x86_64.tar.gz`
- `hoarder-macos-x86_64.tar.gz`
- `hoarder-macos-arm64.tar.gz`
- `hoarder-windows-x86_64.zip`

On a tag push, the workflow publishes those files to the matching GitHub Release. On manual runs, it uploads them as workflow artifacts.

## Local Installer

Unix-like systems:

```bash
curl -fsSL https://raw.githubusercontent.com/Ns2Kracy/hoarder/main/scripts/install.sh | sh
```

Windows PowerShell:

```powershell
iwr https://raw.githubusercontent.com/Ns2Kracy/hoarder/main/scripts/install.ps1 -UseBasicParsing | iex
```

Installer environment variables:

- `HOARDER_REPO`: GitHub repository, default `Ns2Kracy/hoarder`.
- `HOARDER_VERSION`: release tag such as `v0.1.0`, default `latest`.
- `HOARDER_INSTALL_DIR`: install directory, default `$HOME/.local/bin` on Unix and `%USERPROFILE%\.local\bin` on Windows.

## Manual Release Checklist

1. Run local verification: `mise run verify`.
2. Build a local release smoke: `mise run release:smoke`.
3. Tag the release: `git tag v0.1.0 && git push origin v0.1.0`.
4. Wait for the `Release Artifacts` workflow to finish.
5. Download one artifact and run `hoarder --help` before announcing the release.

## Benchmark And Soak Gates

Benchmarks and soak tests are intentionally excluded from default `cargo test --workspace` because they are longer-running and environment-sensitive.

Run the performance benchmark locally:

```bash
mise run bench:local-fs
```

Run the soak test locally:

```bash
mise run soak:local-fs
```
