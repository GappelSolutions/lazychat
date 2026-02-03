# lazychat

A lazygit-inspired TUI for managing AI coding assistant sessions. Monitor, navigate, and interact with your Claude Code sessions in real-time.

> **Note:** This is the original repo. Completely vibe-coded. Fork it, extend it, make it yours — just give credit where credit is due.

> **Recommended:** Use with [oh-my-claudecode](https://github.com/Yeachan-Heo/oh-my-claudecode) for the ultimate Claude Code experience.

![Rust](https://img.shields.io/badge/rust-1.70+-orange.svg)
![License](https://img.shields.io/badge/license-MIT-blue.svg)

```
┌─ Sessions ─────────────┬─ Chat ─────────────────────────────────────┐
│ ▶ fix authentication   │  project/path  │  abc123  │  42 msgs       │
│   <1m ago 156 msgs     │────────────────────────────────────────────│
│ ● add dark mode        │ ▶ You 14:23                                │
│   11m ago 89 msgs      │   Can you fix the login bug?               │
│ ○ refactor api         │                                            │
│   2h ago 234 msgs      │ ◀ Claude 14:24                             │
├─ Files (3) [tree] ─────│   I'll fix the authentication flow...      │
│   src/                 │   └─ Edit [completed]                      │
│ M   auth.rs  +12 -5    │   └─ Bash [completed]                      │
│ M   lib.rs   +3 -1     │                                            │
│ A   tests.rs +45       │ ▶ You 14:25                                │
├─ Todos (2) ────────────│   Perfect, now add tests                   │
│ ▶ Fix login validation │                                            │
│ ○ Add unit tests       │                                            │
└────────────────────────┴────────────────────────────────────────────┘
 j/k: nav │ l: files │ Enter: view │ r: rename │ o: open │ ?: help │ q
```

## Features

- **Real-time session monitoring** - Auto-refreshes every second
- **Git-style file diff viewer** - See changes with syntax highlighting
- **Embedded terminal** - Open Claude directly within the TUI
- **Session status indicators** - Know when Claude is working, idle, or waiting
- **Todo tracking** - View and scroll through session todos
- **File tree view** - Toggle between tree and flat file lists
- **Vim-style navigation** - Familiar keybindings for power users

## Installation

### From source

```bash
git clone https://github.com/gappelsolutions/lazychat
cd lazychat
cargo install --path .
```

### Build only

```bash
cargo build --release
./target/release/lazychat
```

## Quick Start

```bash
# Launch lazychat
lazychat

# Navigate with vim keys
j/k     # Move up/down
h/l     # Switch panels
Enter   # Fullscreen view
Esc     # Go back
?       # Help
```

## Keybindings

### Navigation

| Key                 | Action                                  |
| ------------------- | --------------------------------------- |
| `j` / `k`           | Move down / up                          |
| `h` / `l`           | Switch panels / Jump between diff hunks |
| `g` / `G`           | Go to top / bottom                      |
| `Ctrl+u` / `Ctrl+d` | Page up / down                          |
| `Tab`               | Toggle sidebar ↔ detail focus          |
| `Enter`             | Fullscreen current view                 |
| `Esc`               | Back / Exit fullscreen                  |

### Sessions

| Key | Action                                   |
| --- | ---------------------------------------- |
| `o` | Open session in embedded Claude terminal |
| `n` | Start new Claude session                 |
| `r` | Rename session (custom name override)    |

### Files

| Key | Action                  |
| --- | ----------------------- |
| `f` | Filter files by name    |
| `t` | Toggle tree / flat view |

### General

| Key      | Action                 |
| -------- | ---------------------- |
| `?`      | Toggle help            |
| `q`      | Quit                   |
| `Ctrl+q` | Exit embedded terminal |

## Session Status Indicators

| Icon | Color   | Meaning                                 |
| ---- | ------- | --------------------------------------- |
| `▶` | Cyan    | Working - Claude is actively processing |
| `▶` | Green   | Active - Recent activity (<2 min)       |
| `●`  | Yellow  | Idle - Waiting (2-30 min)               |
| `○`  | Gray    | Inactive - No recent activity (>30 min) |
| `◆`  | Magenta | Waiting - Needs user input (via hooks)  |

## Configuration

Lazychat reads Claude Code data from `~/.claude/`.

### Theme Configuration

Customize colors by creating a config file at `~/.config/lazychat/config.toml` or `~/.lazychat.toml`:

```toml
[theme]
border = "#5c6370"
border_active = "#98c379"
selected_bg = "#1e3250"

# Status colors
status_working = "#56b6c2"
status_active = "#98c379"
status_idle = "#e5c07b"
status_inactive = "#5c6370"
status_waiting = "#c678dd"

# Diff colors
diff_add = "#98c379"
diff_remove = "#e06c75"
diff_hunk = "#61afef"
```

See `config.example.toml` for preset themes (Dracula, Nord, Gruvbox, Tokyo Night).

### Real-time Status with Hooks

For more accurate session status, add these hooks to your `~/.claude/settings.json`:

```json
{
  "hooks": {
    "PreToolUse": [
      {
        "matcher": "",
        "hooks": [
          {
            "type": "command",
            "command": "mkdir -p ~/.claude/session-state && jq -r '.session_id' | xargs -I{} sh -c 'echo working > ~/.claude/session-state/{}.state'"
          }
        ]
      }
    ],
    "Notification": [
      {
        "matcher": "",
        "hooks": [
          {
            "type": "command",
            "command": "jq -r '.session_id' | xargs -I{} sh -c 'echo waiting > ~/.claude/session-state/{}.state'"
          }
        ]
      }
    ],
    "Stop": [
      {
        "matcher": "",
        "hooks": [
          {
            "type": "command",
            "command": "jq -r '.session_id' | xargs -I{} rm -f ~/.claude/session-state/{}.state"
          }
        ]
      }
    ]
  }
}
```

## Architecture

```
src/
├── main.rs          # Entry point, terminal setup
├── app.rs           # Application state & logic
├── events.rs        # Keyboard event handling
├── terminal.rs      # Embedded terminal (PTY)
├── data/
│   ├── mod.rs       # Data structures
│   └── claude.rs    # Claude Code data loading
└── ui/
    ├── mod.rs       # Main UI layout & panels
    └── sessions.rs  # Session list & detail views
```

### Key Components

| Module             | Purpose                                                         |
| ------------------ | --------------------------------------------------------------- |
| `App`              | Central state container - sessions, selection, scroll positions |
| `ClaudeData`       | Async loader for Claude session/message data                    |
| `EmbeddedTerminal` | PTY wrapper for running Claude inside TUI                       |
| `Focus`            | Enum tracking which panel has keyboard focus                    |

## Extending Lazychat

### Adding New AI CLI Support

Lazychat is designed around Claude Code but can be extended for other AI CLIs:

1. **Data Provider Trait** (planned)

```rust
trait AIDataProvider {
    async fn load_sessions(&self) -> Result<Vec<Session>>;
    async fn load_messages(&self, session: &Session) -> Result<Vec<Message>>;
    fn spawn_terminal(&self, session: &Session) -> Result<Process>;
}
```

2. **Implement for your CLI**

```rust
struct CursorDataProvider { /* ... */ }
struct CopilotDataProvider { /* ... */ }
```

3. **Register in config**

```toml
[providers]
default = "claude"
claude = { data_dir = "~/.claude" }
cursor = { data_dir = "~/.cursor" }
```

### Adding New Views

1. Add focus variant to `Focus` enum in `app.rs`
2. Add draw function in `ui/mod.rs` or new module
3. Add keybindings in `events.rs`
4. Update help popup

### Custom Themes (planned)

```toml
[theme]
border = "#5c6370"
border_active = "#98c379"
selected_bg = "#3e4451"
status_working = "#56b6c2"
status_active = "#98c379"
status_idle = "#e5c07b"
```

## API Reference

### Data Structures

```rust
// Session metadata
pub struct Session {
    pub id: String,
    pub project: String,
    pub description: Option<String>,
    pub custom_name: Option<String>,
    pub status: String,           // "working", "active", "idle", "inactive"
    pub message_count: u64,
    pub todos: Vec<TodoItem>,
}

// Chat message with tool calls
pub struct ChatMessage {
    pub role: String,             // "user" or "assistant"
    pub content: String,
    pub tool_calls: Vec<ToolCall>,
}

// File change with git info
pub struct FileChange {
    pub path: String,
    pub filename: String,
    pub status: FileStatus,       // Modified, Added, Deleted, etc.
    pub additions: u32,
    pub deletions: u32,
}
```

### Events

The event loop in `events.rs` handles all keyboard input. To add new keybindings:

```rust
// In handle_key()
KeyCode::Char('x') => {
    if app.focus == Focus::Sessions {
        // Your action here
    }
}
```

## Contributing

We welcome contributions! See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

### Quick Start for Contributors

```bash
# Clone and build
git clone https://github.com/gappelsolutions/lazychat
cd lazychat
cargo build

# Run in development
cargo run

# Run tests
cargo test

# Check formatting
cargo fmt --check
cargo clippy
```

### Areas for Contribution

- **Multi-provider support** - Add data providers for Cursor, Copilot, etc.
- **Custom themes** - Implement theme configuration system
- **Plugin system** - Allow extending UI with custom panels
- **Session management** - Archive, export, import sessions
- **Search** - Full-text search across messages
- **Filters** - Filter sessions by project, date, status

## Requirements

- Rust 1.70+
- Claude Code CLI (`claude` command in PATH)
- Sessions in `~/.claude/`
- Terminal with Unicode support

## License

MIT - See [LICENSE](LICENSE) for details.

## Acknowledgments

- Inspired by [lazygit](https://github.com/jesseduffield/lazygit)
- Built with [ratatui](https://github.com/ratatui-org/ratatui)
- For use with [Claude Code](https://claude.ai/code)
