# AGENTS.md - vLLM Cluster TUI

This document provides guidelines for agentic coding agents working on this codebase.

## Project Overview

A Rust-based terminal UI for managing a distributed vLLM cluster on DGX Spark nodes. Uses ratatui for TUI, tokio for async runtime, and ssh2 for remote SSH connections.

## Build Commands

```bash
# Build in debug mode
cargo build

# Build in release mode (recommended for deployment)
cargo build --release

# Run the application
cargo run --release

# Run a single test by function name
cargo test test_get_session_name
cargo test test_load_models
cargo test test_app_init

# Run all tests
cargo test

# Run doc tests
cargo test --doc

# Check for errors without building
cargo check

# Format code
cargo fmt

# Lint with clippy
cargo clippy
```

## Code Style Guidelines

### Imports and Module Organization

```rust
// Standard library imports first
use std::{io, time::Duration};

// External crate imports second, alphabetically within each group
use anyhow::Result;
use tokio::sync::mpsc;

// Local module imports last
use app::{App, Action};
use config::{AppConfig, load_models};

// Module declarations at top of file (not inline)
mod app;
mod config;
mod ssh;
mod ui;
```

### Formatting

- Use default rustfmt settings (4-space indent, tab expansion enabled)
- No trailing commas in single-line collections
- Line length: try to stay under 100 characters
- One blank line between function definitions and documentation blocks

### Naming Conventions

| Element | Convention | Example |
|---------|------------|---------|
| Modules | snake_case | `ssh_client` |
| Structs | PascalCase | `AppConfig` |
| Enums | PascalCase | `ConnectState` |
| Functions | snake_case | `handle_start_cluster` |
| Constants | SCREAMING_SNAKE_CASE | `MAX_BUFFER_SIZE` |
| Fields | snake_case | `remote_host` |
| Variables | snake_case | `log_tx` |

### Error Handling

- Use `anyhow::Result` for functions that may fail with context-dependent errors
- Use `with_context(|| ...)` to add context to errors
- Use `?` operator for early returns on errors
- For fallible operations, propagate errors rather than panicking
- In main, use `eprintln!` for user-facing errors

```rust
pub fn load_from_file(path: &str) -> Result<Self> {
    let content = fs::read_to_string(path)?;
    let config = serde_json::from_str(&content)?;
    Ok(config)
}
```

### Documentation

- Document all public functions with `///` doc comments
- Include parameter descriptions and return values
- Document enum variants when they carry meaning
- Keep docs concise but informative

```rust
/// Loads the server configuration from a JSON file.
///
/// # Arguments
/// * `path` - Path to the JSON configuration file
///
/// # Returns
/// Parsed AppConfig or an error
```

### Async Patterns

- Use tokio with async/await syntax
- Spawn blocking SSH work on tokio's blocking thread pool: `tokio::task::spawn_blocking`
- Use mpsc channels for communication between async tasks
- Handle errors gracefully in spawned tasks with appropriate logging

### Tests

- Write inline tests in `#[cfg(test)] mod tests` blocks at module end
- Test both success and error paths where applicable
- Use `tempfile` for creating temporary test files (already in dev-dependencies)

### TUI Development

- All UI rendering happens in `src/ui.rs`
- State management in `src/app.rs`
- SSH operations in `src/ssh.rs`
- Configuration loading in `src/config.rs`
- Main loop coordinates everything in `src/main.rs`

## Key Files

| File | Purpose |
|------|---------|
| `src/main.rs` | Entry point, event loop, action handlers |
| `src/app.rs` | Application state and actions |
| `src/ui.rs` | TUI rendering with ratatui |
| `src/ssh.rs` | SSH client wrapping libssh2 |
| `src/config.rs` | JSON config loading and types |
| `server.json` | Runtime server configuration |
| `models.json` | Model definitions |

## Development Notes

- The application requires `server.json` to exist (copy from `server.template.json`)
- Requires `models.json` for model definitions
- SSH connections use agent, pubkey, then password fallback
- All remote operations run via tmux sessions on the remote host
