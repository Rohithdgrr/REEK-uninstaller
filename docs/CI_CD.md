# CI/CD Pipeline

This document describes the continuous integration and delivery setup for REEK
Ultimate Uninstaller. The pipeline lives in `.github/workflows/ci.yml` and runs
on GitHub Actions.

## Overview

```
push / pull_request (main | develop)
                │
                ▼
   ┌──────┬──────┬──────────┬─────────┬─────────┬──────────┬─────────┬──────────┬──────────────┐
   ▼      ▼      ▼          ▼         ▼         ▼          ▼         ▼          ▼              ▼
[1] Check [2] MSRV [3] Security [4] Build [5] Doc [6] Coverage [7] DocsAudit [8] Fuzz [9] Frontend [10] Tauri
fmt/clippy cargo check cargo-audit cargo build cargo doc cargo-llvm-cov required files fuzz 30s  npm ci/build  cli --version
/test   1.88.0   cargo-deny  --release                   floor 40% SECURITY etc 2 targets  ubuntu/win/mac  ubuntu
ubuntu/win/mac          ubuntu  3 targets ubuntu ubuntu    ubuntu     ubuntu(nightly)
```

Release pipeline lives separately in `.github/workflows/release.yml` (trigger: `v*.*.*` tag) — builds both **cargo** binaries and **Tauri updater artifacts** (`latest.json` + `.sig`), publishes to GitHub Releases for auto-update. See [RELEASING.md](RELEASING.md) and [AUTO_UPDATE.md](AUTO_UPDATE.md).

**Documents & images are never auto-deleted:** `EXCLUDED_DOC_IMAGE_EXTS` (`pdf, doc, ppt, xlsx, jpg, png…`) is enforced in every filesystem/junk/duplicate scanner, even if filename contains the app token. Movies vault is opt-in with video-only allowlist (34 exts).

## Jobs

### 1. `check` — Quality gates

Runs on **ubuntu-latest, windows-latest, macos-latest** (matrix).

| Step | Command | Purpose |
|------|---------|---------|
| Checkout | `actions/checkout` | Fetch source, no credentials persisted |
| Toolchain | `dtolnay/rust-toolchain` stable + `rustfmt`, `clippy` | Toolchain |
| Cache | `Swatinem/rust-cache` | Cargo/build cache across runs |
| Formatting | `cargo fmt --all -- --check` | Enforce rustfmt |
| Clippy | `cargo clippy --workspace --all-targets --all-features -- -D warnings` | Enforce zero warnings |
| Tests | `cargo test --workspace --all-features --locked` | Unit + integration tests |

All three commands must pass on all three OSes before a merge is allowed.

### 2. `msrv` — Minimum Supported Rust Version

Runs on **ubuntu-latest** with toolchain **1.88.0**:

```
cargo check --workspace --all-features --locked
```

Guarantees the code compiles on the declared MSRV (`rust-version = "1.88.0"`
in the workspace manifest), not just on the latest stable.

### 3. `security` — Dependency security gate

Runs on **ubuntu-latest**, `permissions: contents: read`.

| Step | Tool | Gate |
|------|------|------|
| `cargo install cargo-audit` | RustSec advisory DB | Fails on any vulnerability |
| `cargo audit` | — | — |
| `cargo install cargo-deny` | cargo-deny | — |
| `cargo deny check advisories bans licenses sources` | — | Fails on bad licenses, wildcards, unknown registries, unmaintained/yanked deps |

Config lives in `deny.toml` and `audit.toml` at the repository root.

> Note: `cargo install` of the security tools makes this job the slowest. For a
> faster CI, pre-build the tools in a container or use `cargo-binstall`
> (`cargo-binstall cargo-audit cargo-deny`) which is significantly faster than
> `cargo install`. They are kept as `cargo install` here for zero external
> dependencies on the runner.

### 4. `build` — Release binaries

Runs on a matrix of native OS + target:

| OS | Target |
|----|--------|
| ubuntu-latest | x86_64-unknown-linux-gnu |
| windows-latest | x86_64-pc-windows-msvc |
| macos-latest | aarch64-apple-darwin |

```
cargo build --workspace --release --target ${{ matrix.target }} --locked
```

Artifacts (`reek`, `reek-tui`) are uploaded with `actions/upload-artifact`
as `reek-<target>` per platform.

### 5. `doc` — Documentation build

Runs on **ubuntu-latest**:

```
cargo doc --workspace --all-features --no-deps --locked
```

### 6. `coverage` — Regression floor

Runs on **ubuntu-latest**, `permissions: contents: read`.

| Step | Tool | Gate |
|------|------|------|
| Toolchain | `dtolnay/rust-toolchain` + `llvm-tools-preview` | Provides `llvm-profdata`/`llvm-cov` |
| `cargo install cargo-llvm-cov --locked --version 0.8.7` | cargo-llvm-cov | Pinned tool version |
| `cargo llvm-cov --workspace --all-features --lcov --output-path lcov.info` | — | Generates `lcov.info` (uploaded as an artifact) |
| `cargo llvm-cov report --fail-under-lines 40` | — | Regression floor 40% (raised from 20% after fuzz + tempfile tests; target 80%+) |

### 7. `docs-audit` — Documentation presence

Checks `SECURITY.md`, `docs/SECURITY.md`, `docs/ARCHITECTURE.md`, `docs/CI_CD.md`, `docs/RELEASING.md`, `CONTRIBUTING.md`, `CHANGELOG.md` exist and contain required sections. Fails build if missing — enforces recommendation E.

### 8. `fuzz` — Fuzz smoke

Runs on `nightly`, 30s per target:

```
cargo fuzz run parse_command -- -max_total_time=30 -max_len=128
cargo fuzz run protected_path -- -max_total_time=30 -max_len=128
```

Targets live in `fuzz/fuzz_targets/`. Unit-level fuzz (500 random strings) also runs in `cargo test` without nightly.

Local equivalent:

```bash
rustup component add llvm-tools-preview
cargo install cargo-llvm-cov --locked --version 0.8.7
make coverage        # write lcov.info
make coverage-report # print per-crate summary
```

## CI/CD security

- Every third-party action is pinned to an **immutable commit SHA** with the
  version tag as a trailing comment. Tags can be moved; SHAs cannot, preventing
  a compromised tag from silently changing your build.
- All jobs use **least-privilege** `permissions: contents: read`. No secrets or
  write tokens are made available.
- `concurrency` cancels superseded runs on the same ref, wasting fewer RUs and
  avoiding stale-result races.
- `--locked` on all cargo invocations means CI resolves dependencies exactly as
  committed in `Cargo.lock` — a missing lockfile update becomes an error rather
  than a silent dependency drift.
- `persist-credentials: false` on checkout prevents the runner from carrying a
  token that a compromised build step could exfiltrate.

## Updating an action pin

To bump a pinned action, resolve the new tag to its SHA:

```bash
git ls-remote https://github.com/<owner>/<repo>.git refs/tags/<tag>
```

Then update `uses: <owner>/<repo>@<sha> # <tag>` and keep the SHA and comment
consistent. Recommended: enable GitHub Dependabot for
`github-actions` and `cargo` ecosystems (see `.github/dependabot.yml` if present).

### 9. `frontend` — Vite + TS build (added 2026-09)

Runs inside the `check` job after cargo cache (on ubuntu/win/macos matrix):

| Step | Command | Purpose |
|------|---------|---------|
| Setup Node | `actions/setup-node@v4` `node 20` + `cache: npm` | Frontend toolchain |
| Install deps | `npm ci` | Reproducible install from `package-lock.json` |
| Build | `npm run build` (`tsc && vite build`) | Type-check + production bundle (`dist/`) — fails on TS errors |
| Tauri CLI | `npx --yes @tauri-apps/cli@latest --version` | Validates `src-tauri/tauri.conf.json` + updater config |

Ensures the desktop UI (Movies vault, Dev Cleaner, SuccessTickDialog) compiles on all OSes before merge.

### 10. `tauri` — Updater artifacts (release only)

Runs only on tag push `v*.*.*` in `.github/workflows/release.yml` (matrix `ubuntu-latest`, `windows-latest`, `macos-latest`):

| Step | Tool | Purpose |
|------|------|---------|
| Setup Node + Rust | `actions/setup-node` + `dtolnay/rust-toolchain` | Toolchain |
| Install deps | `npm ci` | Frontend |
| Linux deps | `apt-get install libwebkit2gtk-4.1-dev …` | WebKit for Tauri on Linux |
| Build Tauri | `tauri-apps/tauri-action@v0` `args: ""` (or `--target aarch64-apple-darwin` on macOS) + `TAURI_SIGNING_PRIVATE_KEY` from secrets | Produces `src-tauri/target/release/bundle/**/*` (`*.msi`, `*.exe`, `*.dmg`, `*.AppImage`) + `latest.json` + `.sig` |
| Upload | `actions/upload-artifact` `tauri-bundle-${os}` | Saved for `release` job to attach to GitHub Release |

The `release` job then downloads both `reek-<target>` (cargo SBOM) and `tauri-bundle-*` artifacts and publishes them together with `latest.json` to GitHub Releases. The endpoint `https://github.com/Rohithdgrr/REEK-uninstaller/releases/latest/download/latest.json` is what the app polls — see `docs/AUTO_UPDATE.md`.

## Local equivalent (Makefile)

Run the same gates locally without GitHub:

```bash
make ci        # test + clippy + fmt-check (everything the check job runs)
make fmt-check # formatting only
make clippy    # clippy with -D warnings
npm ci && npm run build   # frontend (also runs in CI check job)
npx @tauri-apps/cli --version  # Tauri config check
```

## Releases

Releases are cut from `main` via git tags (see `docs/RELEASING.md` and `docs/AUTO_UPDATE.md`). Tag `v0.1.0` triggers `.github/workflows/release.yml` which does **two builds in parallel**:

1. `build` — `cargo build --release` per target + SBOM (`cargo auditable`) + `SHA256SUMS` + OIDC attestations.
2. `tauri` — `tauri-action` builds the desktop app + updater artifacts (`latest.json`, `.sig`, `.msi/.dmg`) using `TAURI_SIGNING_PRIVATE_KEY` secret.

Both artifact sets are merged in the `release` job (`actions/download-artifact` `merge-multiple: true`) and published together via `softprops/action-gh-release` (`files: dist/**/*`). The `latest.json` URL is the auto-update endpoint. See `packaging/homebrew`, `packaging/winget`, `packaging/aur` for downstream recipes.