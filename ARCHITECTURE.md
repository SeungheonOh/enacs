# Enacs Architecture

**Enacs** = Emacs-like editor, Not Actually Capable of Scripting

## 1. High-Level Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                         Frontends                                │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐           │
│  │   Terminal   │  │     GUI      │  │    (Future)  │           │
│  │   Frontend   │  │   Frontend   │  │              │           │
│  └──────┬───────┘  └──────┬───────┘  └──────────────┘           │
└─────────┼─────────────────┼─────────────────────────────────────┘
          │                 │
          ▼                 ▼
┌─────────────────────────────────────────────────────────────────┐
│                      Editor Core                                 │
│  ┌─────────────────────────────────────────────────────────┐    │
│  │                    Editor State                          │    │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐      │    │
│  │  │   Buffer    │  │   Window    │  │  Minibuffer │      │    │
│  │  │   Manager   │  │   Manager   │  │             │      │    │
│  │  └─────────────┘  └─────────────┘  └─────────────┘      │    │
│  └─────────────────────────────────────────────────────────┘    │
│                                                                  │
│  ┌─────────────────────────────────────────────────────────┐    │
│  │                  Command System                          │    │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐      │    │
│  │  │  Keybinding │  │   Command   │  │   Command   │      │    │
│  │  │  Resolver   │  │  Registry   │  │  Dispatcher │      │    │
│  │  └─────────────┘  └─────────────┘  └─────────────┘      │    │
│  └─────────────────────────────────────────────────────────┘    │
│                                                                  │
│  ┌─────────────────────────────────────────────────────────┐    │
│  │                  Editing Primitives                      │    │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐      │    │
│  │  │  Kill Ring  │  │    Undo     │  │    Mark     │      │    │
│  │  │             │  │   System    │  │   System    │      │    │
│  │  └─────────────┘  └─────────────┘  └─────────────┘      │    │
│  └─────────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────────┘
          │
          ▼
┌─────────────────────────────────────────────────────────────────┐
│                       Buffer Layer                               │
│  ┌─────────────────────────────────────────────────────────┐    │
│  │                      Buffer                              │    │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐      │    │
│  │  │    Rope     │  │   Cursors   │  │    Marks    │      │    │
│  │  │   (Text)    │  │  (Multi)    │  │             │      │    │
│  │  └─────────────┘  └─────────────┘  └─────────────┘      │    │
│  └─────────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────────┘
```

## 2. Core Data Structures

### 2.1 Text Storage: Rope

**Choice Justification:**

We use a **Rope** data structure for text storage. Rationale:

| Operation | Gap Buffer | Piece Table | Rope |
|-----------|------------|-------------|------|
| Insert at cursor | O(1) amortized | O(1) | O(log n) |
| Insert at arbitrary position | O(n) | O(1) | O(log n) |
| Delete | O(n) worst | O(1) | O(log n) |
| Index by position | O(1) | O(log n) | O(log n) |
| Index by line | O(n) | O(n) | O(log n) with augmentation |
| Memory efficiency | Poor for multi-cursor | Excellent | Good |
| Undo integration | Complex | Natural | Natural |

**Key advantages of Rope for our requirements:**

1. **Multi-cursor support**: Gap buffer requires O(n) gap movement between cursors. Rope allows O(log n) edits at any position.
2. **Large file handling**: Rope can lazily load chunks; memory proportional to loaded portions.
3. **Line indexing**: Augmented rope tracks line counts per node, enabling O(log n) line lookup.
4. **Undo integration**: Persistent/immutable rope variants allow structural sharing for undo snapshots.

We'll use the `ropey` crate initially, with potential for custom implementation if needed.

### 2.2 Position Representation

```rust
/// Absolute byte offset in buffer
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ByteOffset(pub usize);

/// Character (grapheme cluster) offset
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]  
pub struct CharOffset(pub usize);

/// Line and column position (both 0-indexed internally, 1-indexed for display)
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Position {
    pub line: usize,
    pub column: usize,  // Column in characters, not bytes
}
```

### 2.3 Cursor

```rust
pub struct Cursor {
    /// Current position (character offset)
    pub position: CharOffset,
    
    /// Goal column for vertical movement (to preserve column across short lines)
    pub goal_column: Option<usize>,
    
    /// Mark for this cursor (if set)
    pub mark: Option<CharOffset>,
    
    /// Whether the region (point to mark) is active
    pub mark_active: bool,
}

pub struct CursorSet {
    /// Primary cursor (always exists)
    pub primary: Cursor,
    
    /// Additional cursors, sorted by position
    pub secondary: Vec<Cursor>,
}
```

### 2.4 Buffer

```rust
pub struct Buffer {
    /// Unique buffer ID
    pub id: BufferId,
    
    /// Buffer name (e.g., "*scratch*", "file.txt")
    pub name: String,
    
    /// Associated file path, if any
    pub file_path: Option<PathBuf>,
    
    /// Text content
    pub text: Rope,
    
    /// All cursors in this buffer
    pub cursors: CursorSet,
    
    /// Buffer-local mark ring
    pub mark_ring: MarkRing,
    
    /// Modification flag
    pub modified: bool,
    
    /// Read-only flag
    pub read_only: bool,
    
    /// Undo history
    pub undo_history: UndoHistory,
    
    /// Major mode (static, not programmable)
    pub mode: BufferMode,
}
```

### 2.5 Kill Ring

```rust
/// Global kill ring (shared across buffers, like Emacs)
pub struct KillRing {
    /// Ring buffer of killed text
    entries: VecDeque<String>,
    
    /// Maximum entries to retain
    capacity: usize,
    
    /// Current yank pointer (for M-y cycling)
    yank_pointer: usize,
    
    /// Whether last command was a kill (for appending)
    last_was_kill: bool,
}
```

### 2.6 Undo System

Emacs uses a linear undo model where undo itself is undoable. We implement this:

```rust
pub enum UndoEntry {
    /// Text was inserted at position
    Insert { position: CharOffset, text: String },
    
    /// Text was deleted from position  
    Delete { position: CharOffset, text: String },
    
    /// Cursor positions changed
    CursorMove { before: CursorSet, after: CursorSet },
    
    /// Boundary marker (groups operations)
    Boundary,
}

pub struct UndoHistory {
    /// All undo entries (including undos themselves)
    entries: Vec<UndoEntry>,
    
    /// Current position in history
    position: usize,
    
    /// Whether we're in an undo sequence
    in_undo_sequence: bool,
}
```

## 3. Command System

### 3.1 Command Definition

```rust
pub type CommandFn = fn(&mut EditorState, &CommandContext) -> CommandResult;

pub struct Command {
    /// Canonical name (e.g., "forward-char")
    pub name: &'static str,
    
    /// Function to execute
    pub execute: CommandFn,
    
    /// Whether command preserves/uses the mark
    pub mark_behavior: MarkBehavior,
    
    /// Whether this is a kill command (for kill ring appending)
    pub is_kill: bool,
    
    /// Numeric argument handling
    pub arg_handling: ArgHandling,
}

pub struct CommandContext {
    /// Numeric prefix argument (C-u)
    pub prefix_arg: Option<PrefixArg>,
    
    /// Universal argument count
    pub universal_arg: Option<i32>,
    
    /// Last command executed (for command chaining logic)
    pub last_command: Option<&'static str>,
}
```

### 3.2 Keybinding Resolution

Emacs keybindings form a tree structure with prefix keys:

```rust
pub enum KeyBinding {
    /// Direct command binding
    Command(&'static str),
    
    /// Prefix key leading to more bindings
    Prefix(KeyMap),
    
    /// Unbound
    Unbound,
}

pub struct KeyMap {
    bindings: HashMap<KeyEvent, KeyBinding>,
    
    /// Parent keymap for inheritance
    parent: Option<Box<KeyMap>>,
}

pub struct KeyResolver {
    /// Global keymap
    global: KeyMap,
    
    /// Current accumulated key sequence
    pending_keys: Vec<KeyEvent>,
    
    /// Current keymap being searched
    current_map: Option<KeyMap>,
}
```

**Key Event Representation:**

```rust
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct KeyEvent {
    pub key: Key,
    pub modifiers: Modifiers,
}

pub enum Key {
    Char(char),
    Function(u8),  // F1-F12
    Backspace,
    Tab,
    Enter,
    Escape,
    Up, Down, Left, Right,
    Home, End, PageUp, PageDown,
    Insert, Delete,
}

bitflags! {
    pub struct Modifiers: u8 {
        const CTRL  = 0b0001;
        const META  = 0b0010;  // Alt on most systems
        const SHIFT = 0b0100;
        const SUPER = 0b1000;
    }
}
```

### 3.3 Command Dispatch Flow

```
Input Event
    │
    ▼
┌────────────────┐
│  Key Resolver  │
│  (accumulate   │
│   prefix keys) │
└───────┬────────┘
        │
        ▼
┌────────────────┐     ┌────────────────┐
│ Command Lookup │────▶│    Command     │
│  (by name)     │     │   Registry     │
└───────┬────────┘     └────────────────┘
        │
        ▼
┌────────────────┐
│    Command     │
│   Execution    │
│  (applies to   │
│  all cursors)  │
└───────┬────────┘
        │
        ▼
┌────────────────┐
│  Post-Command  │
│   Hooks        │
│ (undo boundary,│
│  mark update)  │
└────────────────┘
```

## 4. Window Model

```rust
pub struct Window {
    /// Window ID
    pub id: WindowId,
    
    /// Buffer being displayed
    pub buffer_id: BufferId,
    
    /// Top-left position of window in frame
    pub origin: (u16, u16),
    
    /// Size of window (width, height in characters)
    pub size: (u16, u16),
    
    /// First visible line (scroll position)
    pub scroll_line: usize,
    
    /// Horizontal scroll offset
    pub scroll_column: usize,
}

pub struct Frame {
    /// All windows in this frame
    pub windows: Vec<Window>,
    
    /// Currently selected window
    pub selected_window: WindowId,
    
    /// Window layout tree
    pub layout: WindowLayout,
}

pub enum WindowLayout {
    Leaf(WindowId),
    HSplit { left: Box<WindowLayout>, right: Box<WindowLayout>, ratio: f32 },
    VSplit { top: Box<WindowLayout>, bottom: Box<WindowLayout>, ratio: f32 },
}
```

## 5. Frontend Interface

```rust
pub trait Frontend {
    /// Initialize the frontend
    fn init(&mut self) -> Result<(), FrontendError>;
    
    /// Shutdown the frontend
    fn shutdown(&mut self) -> Result<(), FrontendError>;
    
    /// Get terminal/window size
    fn size(&self) -> (u16, u16);
    
    /// Poll for input event (non-blocking)
    fn poll_event(&mut self, timeout: Duration) -> Option<InputEvent>;
    
    /// Render the current editor state
    fn render(&mut self, state: &RenderState) -> Result<(), FrontendError>;
    
    /// Ring the bell / flash screen
    fn bell(&mut self);
}

pub struct RenderState<'a> {
    pub windows: &'a [WindowRenderData],
    pub modeline: &'a str,
    pub minibuffer: Option<&'a MinibufferState>,
    pub message: Option<&'a str>,
}

pub struct WindowRenderData {
    pub buffer_lines: Vec<RenderedLine>,
    pub cursors: Vec<(u16, u16)>,  // Screen positions
    pub regions: Vec<(Position, Position)>,  // Highlighted regions
    pub origin: (u16, u16),
    pub size: (u16, u16),
}
```

## 6. Multi-Cursor Semantics

### 6.1 Cursor Ordering

Cursors are always maintained in sorted order by position. After any edit:
1. Apply edit at each cursor position (in reverse order to preserve positions)
2. Merge overlapping cursors
3. Re-sort if necessary

### 6.2 Region Semantics with Multi-Cursor

Each cursor has its own mark and region. Operations that act on regions (kill-region, etc.) operate on each cursor's region independently.

### 6.3 Conflict Resolution

When edits from different cursors would overlap:
- Insertions: Each cursor inserts independently; text accumulates
- Deletions: Merged into single deletion spanning all affected ranges
- After operation: Cursors that would occupy same position are merged

## 7. Module Structure

```
enacs/
├── Cargo.toml
├── src/
│   ├── lib.rs              # Library root
│   ├── main.rs             # Binary entry point
│   │
│   ├── core/               # Core editing engine
│   │   ├── mod.rs
│   │   ├── buffer.rs       # Buffer implementation
│   │   ├── cursor.rs       # Cursor and multi-cursor
│   │   ├── rope.rs         # Rope wrapper/extensions
│   │   ├── position.rs     # Position types
│   │   ├── mark.rs         # Mark and region
│   │   ├── kill_ring.rs    # Kill ring
│   │   └── undo.rs         # Undo system
│   │
│   ├── commands/           # Command implementations
│   │   ├── mod.rs
│   │   ├── registry.rs     # Command registry
│   │   ├── motion.rs       # Movement commands
│   │   ├── editing.rs      # Insert/delete commands
│   │   ├── kill_yank.rs    # Kill and yank
│   │   ├── buffer_cmds.rs  # Buffer operations
│   │   ├── window_cmds.rs  # Window operations
│   │   └── file_cmds.rs    # File I/O commands
│   │
│   ├── keybinding/         # Keybinding system
│   │   ├── mod.rs
│   │   ├── keymap.rs       # Keymap structure
│   │   ├── resolver.rs     # Key sequence resolution
│   │   ├── default.rs      # Default Emacs bindings
│   │   └── key.rs          # Key event types
│   │
│   ├── state/              # Editor state management
│   │   ├── mod.rs
│   │   ├── editor.rs       # Main editor state
│   │   ├── buffer_mgr.rs   # Buffer manager
│   │   ├── window_mgr.rs   # Window manager
│   │   └── minibuffer.rs   # Minibuffer state
│   │
│   └── frontend/           # Frontend implementations
│       ├── mod.rs
│       ├── traits.rs       # Frontend trait
│       └── terminal/       # Terminal frontend
│           ├── mod.rs
│           ├── render.rs   # Rendering
│           └── input.rs    # Input handling
│
└── tests/                  # Integration tests
    ├── motion_tests.rs
    ├── editing_tests.rs
    └── keybinding_tests.rs
```

## 8. Implementation Phases

### Phase 1: Foundation (Current)
- [ ] Project setup
- [ ] Rope integration
- [ ] Buffer with single cursor
- [ ] Basic insert/delete
- [ ] Terminal frontend (minimal)

### Phase 2: Core Editing
- [ ] All movement commands
- [ ] Mark and region
- [ ] Kill ring
- [ ] Undo system
- [ ] Multi-cursor basics

### Phase 3: Command Infrastructure
- [ ] Command registry
- [ ] Keybinding resolver
- [ ] Default Emacs keybindings
- [ ] Prefix argument handling

### Phase 4: Buffer/Window Management
- [ ] Multiple buffers
- [ ] Buffer switching
- [ ] Window splitting
- [ ] Minibuffer

### Phase 5: File Operations
- [ ] File open/save
- [ ] Dirty buffer handling
- [ ] Auto-save (optional)

### Phase 6: Polish
- [ ] Complete keybinding coverage
- [ ] Edge cases
- [ ] Performance optimization
- [ ] Documentation

## 9. Design Decisions Log

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Text storage | Rope (ropey crate) | O(log n) edits, multi-cursor friendly, line indexing |
| Undo model | Linear (Emacs-style) | Behavioral fidelity requirement |
| Cursor storage | Per-buffer CursorSet | Multi-cursor with primary/secondary distinction |
| Kill ring | Global singleton | Matches Emacs semantics |
| Command dispatch | Static registry | No Lisp, all commands built-in |
| Frontend abstraction | Trait-based | Allows terminal + future GUI |
| Keybinding | Tree of HashMaps | Efficient prefix key handling |

## 10. Emacs Commands: Initial Scope

### Movement (Phase 2)
- `forward-char` (C-f)
- `backward-char` (C-b)
- `forward-word` (M-f)
- `backward-word` (M-b)
- `next-line` (C-n)
- `previous-line` (C-p)
- `move-beginning-of-line` (C-a)
- `move-end-of-line` (C-e)
- `beginning-of-buffer` (M-<)
- `end-of-buffer` (M->)
- `forward-paragraph` (M-})
- `backward-paragraph` (M-{)
- `scroll-up-command` (C-v)
- `scroll-down-command` (M-v)
- `recenter-top-bottom` (C-l)

### Editing (Phase 2)
- `self-insert-command`
- `delete-char` (C-d)
- `delete-backward-char` (DEL)
- `kill-line` (C-k)
- `kill-word` (M-d)
- `backward-kill-word` (M-DEL)
- `kill-region` (C-w)
- `copy-region-as-kill` (M-w)
- `yank` (C-y)
- `yank-pop` (M-y)
- `undo` (C-/, C-x u)
- `newline` (RET)
- `open-line` (C-o)
- `transpose-chars` (C-t)

### Mark (Phase 2)
- `set-mark-command` (C-SPC)
- `exchange-point-and-mark` (C-x C-x)
- `mark-word` (M-@)
- `mark-paragraph` (M-h)
- `mark-whole-buffer` (C-x h)

### Buffer (Phase 4)
- `switch-to-buffer` (C-x b)
- `kill-buffer` (C-x k)
- `list-buffers` (C-x C-b)

### File (Phase 5)
- `find-file` (C-x C-f)
- `save-buffer` (C-x C-s)
- `write-file` (C-x C-w)

### Window (Phase 4)
- `split-window-below` (C-x 2)
- `split-window-right` (C-x 3)
- `delete-window` (C-x 0)
- `delete-other-windows` (C-x 1)
- `other-window` (C-x o)

### Misc
- `keyboard-quit` (C-g)
- `execute-extended-command` (M-x)
