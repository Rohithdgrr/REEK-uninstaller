# Installation Instructions

## Prerequisites

REEK Ultimate Uninstaller requires Rust toolchain to build and run.

## Installing Rust on Windows

### Option 1: Using rustup (Recommended)

1. Download the rustup installer from: https://rustup.rs/
2. Run the installer and follow the prompts
3. Restart your terminal/command prompt after installation

### Option 2: Using PowerShell

Run the following command in PowerShell:

```powershell
Invoke-WebRequest -Uri https://win.rustup.rs/x86_64 -OutFile rustup-init.exe
.\rustup-init.exe
```

### Option 3: Using Chocolatey

If you have Chocolatey installed:

```powershell
choco install rust
```

### Option 4: Using Scoop

If you have Scoop installed:

```powershell
scoop install rustup
```

## Verify Installation

After installation, verify Rust is installed by running:

```powershell
rustc --version
cargo --version
```

## Building the Project

Once Rust is installed, navigate to the project directory and run:

```powershell
cd C:\Users\rohit\Music\REEK
cargo build --workspace
```

## Building Release Binaries

For optimized release builds:

```powershell
cargo build --workspace --release
```

The binaries will be located at:
- CLI: `target\release\reek.exe`
- TUI: `target\release\reek-tui.exe`

## Running Tests

```powershell
cargo test --workspace --all-features
```

## Running the Application

### CLI Version
```powershell
cargo run --bin reek -- --help
```

### TUI Version
```powershell
cargo run --bin reek-tui
```

## Troubleshooting

### If cargo is not recognized after installation:
1. Close and reopen your terminal
2. Or add Rust to your PATH manually:
   - Add `%USERPROFILE%\.cargo\bin` to your PATH environment variable

### If you encounter build errors:
1. Ensure you have the latest Rust version: `rustup update`
2. Install required build tools (Visual Studio Build Tools for Windows)
3. Run: `cargo clean` then try building again

## Additional Requirements

For Windows-specific features, you may need:
- Visual Studio Build Tools (for C++ compilation)
- Windows SDK

Download from: https://visualstudio.microsoft.com/downloads/#build-tools-for-visual-studio-2022
