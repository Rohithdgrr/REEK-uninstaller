# Releasing

This document describes how to cut a release of REEK Ultimate Uninstaller.

> The current codebase is at **0.1.0**. Releases are tagged `v0.1.0`,
> `v0.1.1`, etc. following [Semantic Versioning](https://semver.org/).

## Prerequisites

- Write access to the repository.
- The Rust toolchain (any recent stable).
- A passing CI run on `main` (all jobs in `.github/workflows/ci.yml`).

## Release checklist

### 1. Update the changelog

Add a `## [x.y.z] - YYYY-MM-DD` section at the top of `CHANGELOG.md` and move
any `[Unreleased]` entries into it. Follow the existing format
(`### Added`, `### Changed`, `### Fixed`, `### Security`).

### 2. Bump the version

The workspace version is defined once in the root `Cargo.toml`
(`[workspace.package] version`). Each crate uses `version = "0.1.0"` — update
them to match:

- `crates/greek-common/Cargo.toml`
- `crates/greek-core/Cargo.toml`
- `crates/greek-tui/Cargo.toml`
- `crates/greek-cli/Cargo.toml`
- `crates/greek-windows/Cargo.toml`
- `crates/greek-platform/Cargo.toml`

All internal path dependencies that carry a `version` (e.g.
`greek-common = { path = "../greek-common", version = "x" }`) must also be
bumped to match.

```bash
# Update all version strings uniformly:
#   version = "0.1.0" → version = "0.1.1"
```

### 3. Update the lockfile

```bash
cargo check --workspace --all-features --locked
```

Resolves and records the new workspace versions in `Cargo.lock`, which is
committed.

### 4. Run the full verification locally

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features --locked
cargo audit
cargo deny check advisories bans licenses sources
cargo doc --workspace --all-features --no-deps
```

### 5. Open a release PR

Create a branch, commit the changes, and open a PR against `main` with the
release checklist. All CI jobs must pass.

### 6. Tag and release

After the PR merges and `main` is green:

```bash
git tag v0.1.1
git push origin v0.1.1
```

` .github/workflows/release.yml` triggers on `v*.*.*` tags:

- Builds `reek` + `reek-tui` for all three targets (`x86_64-unknown-linux-gnu`, `x86_64-pc-windows-msvc`, `aarch64-apple-darwin`).
- Generates SBOM via `cargo auditable` + `Cargo.lock`, SHA256SUMS, and SLSA build provenance via `actions/attest-build-provenance` (OIDC, no long-lived signing keys).
- Creates a GitHub Release with all artifacts and auto-generated notes from `CHANGELOG.md`.

## What the release contains

The `release` workflow produces per target:

| Target | Binary + Artifacts |
|--------|--------|
| x86_64-unknown-linux-gnu | `reek`, `reek-tui`, `Cargo.lock`, `sbom.json`, `SHA256SUMS` |
| x86_64-pc-windows-msvc | `reek.exe`, `reek-tui.exe`, `Cargo.lock`, `sbom.json`, `SHA256SUMS` |
| aarch64-apple-darwin | `reek`, `reek-tui`, `Cargo.lock`, `sbom.json`, `SHA256SUMS` |

Downstream packaging (see `packaging/`):

- **Homebrew**: `packaging/homebrew/reek.rb` — update `sha256` after release, `brew audit --strict`.
- **Winget**: `packaging/winget/greek.reek.yaml` — run `wingetcreate update`.
- **AUR**: `packaging/aur/PKGBUILD` — update `sha256sums`, `makepkg --printsrcinfo`, push to AUR.

Code signing: Windows binaries are signed via Sigstore OIDC (keyless cosign) when `cosign` is added to the release job; EV cert signing can replace this without workflow changes.

### SBOM & Supply Chain

- `Cargo.lock` committed (reproducible builds).
- `cargo auditable` embeds dependency list in binaries (`cargo auditable report`).
- `cargo audit` + `cargo deny` run in both `ci.yml` and `release.yml`.

## Patch / security releases

- For a security fix, backport the fix to the same branch as the last release,
  bump the patch version, and add a `### Security` section to the changelog.
- Coordinate disclosure per [SECURITY.md](../SECURITY.md).