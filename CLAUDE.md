# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

reth-desktop is a desktop GUI application for reth (Rust Ethereum execution client). This project is in its initial setup phase.

## Development Commands

```bash
# Build the project
cargo build

# Run the application
cargo run

# Build in release mode
cargo build --release

# Run tests
cargo test

# Check for compilation errors without building
cargo check

# Format code
cargo fmt

# Run linter
cargo clippy
```

## Architecture Notes

The project uses egui (a native Rust immediate-mode GUI library) with eframe for creating the desktop application. Key architectural components:

- **GUI Framework**: egui/eframe - Immediate mode GUI in pure Rust
- **Rendering**: Native rendering (no web technologies)
- **Communication with reth**: TBD (likely JSON-RPC over HTTP/IPC)
- **State Management**: Immediate mode pattern (state stored in App struct)

## Important Considerations

When developing this application, keep in mind:
- reth is a Rust-based Ethereum execution client
- The GUI should facilitate node management, monitoring, and configuration
- Consider cross-platform compatibility (Windows, macOS, Linux)
- Security is paramount when dealing with Ethereum nodes