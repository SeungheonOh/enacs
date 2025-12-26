# Enacs Development Guide

## Build Commands

```bash
# Build debug
cargo build

# Build release
cargo build --release

# Run tests
cargo test

# Run with clippy
cargo clippy

# Format code
cargo fmt
```

## Architecture Overview

- `src/core/` - Core data structures (Buffer, Cursor, Rope extensions, KillRing, Undo)
- `src/commands/` - All command implementations
- `src/keybinding/` - Key events, keymaps, resolver
- `src/state/` - Editor state management
- `src/frontend/` - Terminal rendering and input

## Adding New Commands

1. Implement the command function in the appropriate module under `src/commands/`
2. Add it to the `all_commands()` function in that module
3. Add keybinding in `src/keybinding/default.rs`

## Code Style

- No comments unless code is complex
- Use Rust idioms
- Follow existing patterns in the codebase
