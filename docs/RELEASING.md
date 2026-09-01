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

### 2. Bump the version (workspace + Tauri)

The workspace version is defined once in the root `Cargo.toml`
(`[workspace.package] version`) **and** in `src-tauri/tauri.conf.json` (`version`) + `package.json` (`version`) + `src-tauri/Cargo.toml` (`version`). Each crate uses `version = "0.1.0"` — update them to match:

- `Cargo.toml` (`[workspace.package]`)
- `src-tauri/tauri.conf.json` (`version`)
- `src-tauri/Cargo.toml` (`version`)
- `package.json` (`version`)
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
# Also: src-tauri/tauri.conf.json, package.json, src-tauri/Cargo.toml
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

` .github/workflows/release.yml` triggers on `v*.*.*` tags and does **two parallel builds**:

- **Cargo build** (`build` job): `reek` + `reek-tui` for all three targets (`x86_64-unknown-linux-gnu`, `x86_64-pc-windows-msvc`, `aarch64-apple-darwin`) + SBOM (`cargo auditable`) + `SHA256SUMS` + SLSA attestations.
- **Tauri build** (`tauri` job, matrix `ubuntu/win/macos`): `npm ci` + `tauri-action` builds the desktop app bundles (`.msi`, `.exe`, `.dmg`, `.AppImage`) **plus updater artifacts** (`latest.json` + `.sig`) signed with `TAURI_SIGNING_PRIVATE_KEY` (from GitHub Secrets). See `docs/AUTO_UPDATE.md` for signing.
- **Release** job merges both artifact sets and creates a GitHub Release with `softprops/action-gh-release`, attaching `latest.json` at `https://github.com/Rohithdgrr/REEK-uninstaller/releases/latest/download/latest.json` — the endpoint polled by `plugin-updater` for auto-update.

**Auto-update signing:** generate once `npm run tauri signer generate -w ~/.tauri/reek.key`, set `TAURI_SIGNING_PRIVATE_KEY` (file contents) and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` in repo Secrets → Actions. The public key goes in `src-tauri/tauri.conf.json:plugins.updater.pubkey`. Without it, the Tauri build still succeeds but `latest.json` is unsigned and updater will not install.

## What the release contains

The `release` workflow publishes **two artifact families** to the same GitHub Release:

**Cargo (CLI/TUI):**

| Target | Binary + Artifacts |
|--------|-------------------|
| x86_64-unknown-linux-gnu | `reek`, `reek-tui`, `Cargo.lock`, `sbom.json`, `SHA256SUMS` |
| x86_64-pc-windows-msvc | `reek.exe`, `reek-tui.exe`, `Cargo.lock`, `sbom.json`, `SHA256SUMS` |
| aarch64-apple-darwin | `reek`, `reek-tui`, `Cargo.lock`, `sbom.json`, `SHA256SUMS` |

**Tauri desktop + updater (auto-update):**

| OS | Bundle + Updater |
|----|------------------|
| ubuntu-latest | `reek-uninstaller_0.1.0_amd64.AppImage`, `.deb`, `latest.json`, `.sig` |
| windows-latest | `REEK-Uninstaller_0.1.0_x64_en-US.msi`, `*.exe`, `latest.json`, `.sig` |
| macos-latest | `REEK.Uninstaller_0.1.0_aarch64.dmg`, `.app.tar.gz`, `latest.json`, `.sig` |

The `latest.json` for each platform is what `tauri-plugin-updater` polls — see `docs/AUTO_UPDATE.md`.

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