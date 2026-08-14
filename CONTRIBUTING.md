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

- Use `cargo fmt` to format code
- Use `cargo clippy` to check for linting issues
- Follow Rust naming conventions
- Add documentation comments to public APIs

## Testing

- Write unit tests for new functionality
- Ensure all tests pass before submitting PRs
- Use `cargo test --workspace --all-features` to run all tests

## Submitting Changes

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests
5. Run `cargo fmt` and `cargo clippy`
6. Submit a pull request

## Project Structure

- `greek-common` - Shared types, errors, and utilities
- `greek-platform` - Platform abstraction traits
- `greek-windows` - Windows-specific implementations
- `greek-core` - Core uninstallation logic
- `greek-cli` - Command-line interface
- `greek-tui` - Terminal user interface

## Safety Considerations

REEK makes significant changes to the system. Always:
- Test uninstall operations carefully
- Implement proper error handling
- Add safety checks for destructive operations
- Document any potentially dangerous operations

## License

By contributing, you agree that your contributions will be licensed under the MIT OR Apache-2.0 license.
