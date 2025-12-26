# Enacs

**Emacs-like editor, Not Actually Capable of Scripting**

A text editor with Emacs keybindings and editing semantics, implemented in Rust. Focuses on behavioral fidelity to Emacs without Emacs Lisp programmability.

## Features

### Emacs Compatibility
- **Cursor Movement**: Character, word, line, sentence, paragraph, buffer navigation
- **Editing**: Insert, delete, kill, yank with proper kill-ring semantics
- **Mark & Region**: Set mark, active regions, region-based operations
- **Keybindings**: Standard Emacs bindings including prefix keys (C-x, M-g)
- **Buffers**: Full buffer abstraction with file-backed and scratch buffers
- **Windows**: Window splitting (vertical/horizontal), navigation
- **Undo**: Emacs-style linear undo

### Modern Additions
- **Multi-cursor editing**: Native support for multiple simultaneous cursors
- **Large file handling**: Rope data structure for efficient editing of large files
- **Terminal UI**: Full terminal-based interface using crossterm

## Building

```bash
cargo build --release
```

## Usage

```bash
# Open with scratch buffer
./target/release/enacs

# Open a file
./target/release/enacs path/to/file.txt
```

## Key Bindings

### Movement
| Key | Command |
|-----|---------|
| C-f | forward-char |
| C-b | backward-char |
| C-n | next-line |
| C-p | previous-line |
| C-a | move-beginning-of-line |
| C-e | move-end-of-line |
| M-f | forward-word |
| M-b | backward-word |
| M-< | beginning-of-buffer |
| M-> | end-of-buffer |
| C-v | scroll-up-command |
| M-v | scroll-down-command |
| C-l | recenter-top-bottom |
| Arrow keys | Movement |
| Alt+Left/Right | Word movement |
| Ctrl+Home/End | Buffer start/end |

### Selection (Shift-Select)
Any movement command with Shift held will start/extend selection:

| Key | Command |
|-----|---------|
| Shift+C-f / Shift+Right | Select forward char |
| Shift+C-b / Shift+Left | Select backward char |
| Shift+C-n / Shift+Down | Select next line |
| Shift+C-p / Shift+Up | Select previous line |
| Shift+C-a / Shift+Home | Select to line start |
| Shift+C-e / Shift+End | Select to line end |
| Shift+M-f / Shift+Alt+Right | Select forward word |
| Shift+M-b / Shift+Alt+Left | Select backward word |
| Shift+M-< | Select to buffer start |
| Shift+M-> | Select to buffer end |
| Ctrl+Shift+Home/End | Select to buffer start/end |

### Editing
| Key | Command |
|-----|---------|
| C-d | delete-char |
| DEL | delete-backward-char |
| C-k | kill-line |
| M-d | kill-word |
| M-DEL | backward-kill-word |
| C-w | kill-region |
| M-w | copy-region-as-kill |
| C-y | yank |
| M-y | yank-pop |
| C-/ | undo |
| C-t | transpose-chars |
| C-o | open-line |

### Mark & Region
| Key | Command |
|-----|---------|
| C-SPC | set-mark-command |
| C-x C-x | exchange-point-and-mark |
| C-x h | mark-whole-buffer |

### Buffer & File
| Key | Command |
|-----|---------|
| C-x C-f | find-file |
| C-x C-s | save-buffer |
| C-x C-w | write-file |
| C-x b | switch-to-buffer |
| C-x k | kill-buffer |
| C-x C-b | list-buffers |

### Window
| Key | Command |
|-----|---------|
| C-x 2 | split-window-below |
| C-x 3 | split-window-right |
| C-x 0 | delete-window |
| C-x 1 | delete-other-windows |
| C-x o | other-window |

### Other
| Key | Command |
|-----|---------|
| C-g | keyboard-quit |
| M-x | execute-extended-command |
| C-x C-c | exit |

## Architecture

See [ARCHITECTURE.md](ARCHITECTURE.md) for detailed design documentation.

### Key Design Decisions

1. **Text Storage**: Rope data structure (via `ropey` crate) for O(log n) editing operations
2. **Multi-cursor**: First-class support with deterministic cursor ordering
3. **Frontend-agnostic**: Core engine separated from terminal rendering
4. **Static commands**: All commands are built-in (no user scripting)

## Project Structure

```
src/
├── core/           # Core data structures (Buffer, Cursor, Rope, KillRing, Undo)
├── commands/       # Command implementations (motion, editing, kill-yank, etc.)
├── keybinding/     # Key event types, keymap, and resolver
├── state/          # Editor state, buffer manager, window manager
└── frontend/       # Terminal frontend (rendering, input)
```

## Testing

```bash
cargo test
```

## License

MIT OR Apache-2.0
