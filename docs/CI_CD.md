# CI/CD Pipeline

This document describes the continuous integration and delivery setup for REEK
Ultimate Uninstaller. The pipeline lives in `.github/workflows/ci.yml` and runs
on GitHub Actions.

## Overview

```
push / pull_request (main | develop)
                │
                ▼
        ┌───────┴───────────┬───────────────────────┬──────────────┬──────────────┐
        ▼                   ▼                       ▼              ▼              ▼
  [1] Check             [2] MSRV                [3] Security    [4] Build      [5] Doc
  fmt / clippy          cargo check                cargo-audit   cargo build    cargo doc
  / test                1.78.0                     cargo-deny    --release
  ubuntu/windows/macos                              ubuntu       3 targets      ubuntu
```

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

Runs on **ubuntu-latest** with toolchain **1.78.0**:

```
cargo check --workspace --all-features --locked
```

Guarantees the code compiles on the declared MSRV (`rust-version = "1.78.0"`
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

## Local equivalent (Makefile)

Run the same gates locally without GitHub:

```bash
make ci        # test + clippy + fmt-check (everything the check job runs)
make fmt-check # formatting only
make clippy    # clippy with -D warnings
```

## Releases

Releases are cut from `main` via git tags (see `docs/RELEASING.md`). A
release tag `v0.1.0` builds the same `build` job output and attaches artifacts
to the GitHub Release. The release pipeline is intentionally not wired into the
PR-triggered `ci.yml`; it is triggered separately on tag pushes.