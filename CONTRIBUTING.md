# Contributing to REEK

Thank you for your interest in contributing to REEK Ultimate Uninstaller!

## Development Setup

1. Clone the repository:
```bash
git clone https://github.com/greek/greek-uninstaller.git
cd greek-uninstaller
```

2. Install Rust (if not already installed):
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

3. Build the project:
```bash
cargo build --workspace
```

4. Run tests:
```bash
cargo test --workspace --all-features
```

## Code Style

- Run `cargo fmt` to format code (rustfmt options live in `rustfmt.toml`).
- Run `cargo clippy` and keep the workspace **warning-free**; CI enforces
  `clippy -- -D warnings`.
- Follow Rust naming conventions and rustdoc comments on public APIs.

## Testing

- Write unit tests for new functionality.
- Integration tests live under each crate's `tests/` directory.
- Run everything before pushing:
  ```bash
  cargo test --workspace --all-features --locked
  ```

## Required checks (all must pass)

These are enforced by CI and runnable locally via `make ci`:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features --locked
```

## Dependency changes

Because REEK ships binaries, dependency hygiene is a security control:

- Keep `Cargo.lock` **committed** — never add it to `.gitignore`.
- Use explicit version requirements, **never `*` wildcards** (denied by
  `cargo-deny`).
- New dependencies must have an OSI-approved license in `deny.toml`'s allow
  list (or be justified with an exception), and must pass:
  ```bash
  cargo audit
  cargo deny check advisories bans licenses sources
  ```

## Submitting Changes

1. Fork the repository.
2. Create a feature branch.
3. Make your changes; add tests.
4. Run `cargo fmt` and `cargo clippy`.
5. Update `CHANGELOG.md` under `[Unreleased]`.
6. Submit a pull request against `main`.

## Project Structure

- `greek-common` — Shared types, errors, and utilities
- `greek-platform` — Linux/macOS platform scanners & helpers
- `greek-windows` — Windows-specific implementations
- `greek-core` — Core uninstallation logic
- `greek-cli` — Headless CLI (`reek`)
- `greek-tui` — Terminal user interface (`reek-tui`)

See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md).

## Safety Considerations

REEK makes significant, **destructive** changes to the system. Always:

- Test uninstall operations carefully — prefer throwaway test apps.
- Add safety checks for destructive operations (protected paths, confirmation).
- Never introduce code that executes shell metacharacters from uninstall
  strings or registry data. Use the existing quote-aware
  `parse_command_string` and `Command::new` without a shell.
- If your change touches deletion logic, re-read [docs/SECURITY.md](docs/SECURITY.md).

## Security Issues

Do **not** report security issues in public issues/PRs. See
[SECURITY.md](SECURITY.md) for the private reporting process.

## License

By contributing, you agree that your contributions will be licensed under the
MIT OR Apache-2.0 license.