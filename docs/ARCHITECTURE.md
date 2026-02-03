# Lazychat Architecture

A terminal UI (TUI) for exploring and interacting with Claude AI sessions, built with Rust and Ratatui.

## High-Level Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                         TERMINAL (PTY)                          │
│  ┌─────────────────────────────────────────────────────────┐    │
│  │  Embedded Terminal (portable_pty + vt100 parser)       │    │
│  │  - Renders Claude AI agent output                      │    │
│  │  - Forwards keyboard input                             │    │
│  └─────────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────────┘
           ▲                                       ▲
           │ (key input)                    (terminal output)
           │                                       │
┌──────────┴───────────────────────────────────────┴──────────────┐
│                         EVENT LOOP                              │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │ run_app(): Async loop polling for keyboard events       │   │
│  │ - 100ms timeout between frames                          │   │
│  │ - Auto-refresh session data every 1 second             │   │
│  │ - Detects session selection changes                     │   │
│  └──────────────────────────────────────────────────────────┘   │
└──────────────────┬──────────────────────────────────────────────┘
                   │
                   ├─ handle_key() ─────────────────────────┐
                   │                                       │
        ┌──────────▼───────────────┐            ┌──────────▼───────┐
        │   App State              │            │  UI Rendering    │
        │  (mutable reference)     │            │  (immutable)     │
        └─────────┬────────────────┘            └──────────────────┘
                  │
         ┌────────▼─────────────────────────────────┐
         │      Data Loading (ClaudeData)           │
         │                                          │
         │  ~/.claude/                              │
         │  ├── projects/*/session.jsonl (chat)     │
         │  ├── history.jsonl (descriptions)        │
         │  ├── tasks/ (todo items)                 │
         │  ├── todos/ (agent state)                │
         │  └── session-state/ (status files)       │
         └────────────────────────────────────────┘
```

## Module Responsibilities

### Core Modules

#### `main.rs`
- Initializes terminal environment (raw mode, alternate screen, mouse capture)
- Parses CLI arguments (watch interval, refresh rate)
- Creates App instance and loads initial data
- Runs event loop
- Cleans up terminal state on exit

**Key Decision**: Uses `tokio::main` for async runtime - allows non-blocking file I/O during event loop.

#### `app.rs` - State Management
The central state struct containing:

**Structural State**
- `sessions: Vec<Session>` - All loaded sessions from ~/.claude/projects
- `agents: Vec<Agent>` - Agent metadata from ~/.claude/todos
- `focus: Focus` - Current panel (Sessions/Files/Todos/Detail)

**View State**
- `session_list_state: ListState` - Selected session index
- `chat_scroll`, `files_scroll`, `todos_scroll` - Scroll positions per panel
- `diff_mode: bool` - Whether detail pane shows diff or chat
- `fullscreen: bool` - Whether detail pane is fullscreen
- `file_tree_mode: bool` - Tree vs flat file view

**Interaction State**
- `renaming: bool`, `rename_buffer` - Session rename input
- `file_filter_active: bool`, `file_filter` - File search
- `terminal_mode: bool`, `embedded_terminal: Option<EmbeddedTerminal>` - PTY session

**Chat & Files**
- `current_messages: Vec<ChatMessage>` - Loaded messages for selected session
- `current_file_changes: Vec<FileChange>` - Git changes extracted from tool calls
- `current_diff: String` - Rendered diff for selected file

**Status Messages**
- `status_message: Option<String>`, `status_is_error: bool` - Transient notifications

**Key Methods**
- `load_data()` - Async: loads all sessions, agents, tasks from ~/.claude
- `load_session_messages()` - Async: loads chat transcript for selected session
- `load_file_diff()` - Async: runs git diff for selected file
- `get_file_changes()` - Async: extracts edited files from messages and gets git status
- Navigation helpers: `list_next/prev`, `scroll_up/down`, `jump_to_*_hunk`
- UI helpers: `toggle_focus`, `start_rename`, `set_status`

#### `events.rs` - Event Handling
Implements the main event loop and key bindings.

**Event Loop** (`run_app`)
```rust
loop {
    // Draw current state
    terminal.draw(|f| ui::draw(f, app))?;

    // Auto-refresh every 1 second
    if last_refresh.elapsed() >= Duration::from_secs(1) {
        app.load_data().await;
    }

    // Detect session selection changes
    if selected_session_changed {
        app.load_session_messages().await;
    }

    // Poll for events (100ms timeout)
    if event::poll(Duration::from_millis(100))? {
        if let Event::Key(key) = event::read()? {
            if handle_key(app, key).await? {
                return Ok(()); // Exit signal
            }
        }
    }
}
```

**Key Binding Architecture**
- Terminal mode (PTY running) - keys forwarded to PTY, not handled locally
- Help popup - only Esc/q handled
- Rename/filter input modes - dedicated input handling
- Normal mode - vim-style navigation (hjkl, jk for scroll, etc.)

**Special Key Translation** (`key_to_bytes`)
- Converts crossterm KeyEvents to terminal escape sequences
- Handles Ctrl+char, special keys (arrows, Page Up/Down, etc.)
- Used when forwarding keys to embedded terminal

### Data Modules

#### `data/mod.rs` - Data Structures

Core types:
```rust
Session {
    id: String,              // Session ID (uuid)
    project: String,         // Full project path (e.g., "Users/name/project")
    project_name: String,    // Last component for display
    description: Option<String>,  // First user message (from history.jsonl)
    custom_name: Option<String>,  // User override via 'r' key
    started_at, last_activity: Option<DateTime<Utc>>,
    message_count: u64,      // Estimated from file size
    status: String,          // "working"/"active"/"idle"/"inactive"/"waiting"
    todos: Vec<TodoItem>,    // Tasks for this session
}

ChatMessage {
    role: String,            // "user" or "assistant"
    content: String,
    timestamp: Option<DateTime<Utc>>,
    tool_calls: Vec<ToolCall>,  // Edit/Write/etc. extracted from assistant message
}

FileChange {
    path: String,            // Full path to file
    filename: String,        // Basename for display
    status: FileStatus,      // Modified/Added/Deleted/Renamed/Untracked
    additions, deletions: u32, // Line counts from git
}
```

#### `data/claude.rs` - Data Loading

Loads data from ~/.claude directory structure:

**Session Loading** (`load_sessions`)
- Scans `~/.claude/projects/*/` directories
- Reads `.jsonl` transcript files
- Extracts file metadata (modification time = last activity)
- Estimates message count from file size (500 bytes per message)
- Determines status by file age:
  - `working` (<10s old): cyan spinner ⟳
  - `active` (<2min): green play ▶
  - `idle` (2-30min): yellow dot ●
  - `inactive` (>30min): gray circle ○

**Message Loading** (`load_session_messages`)
- Parses `.jsonl` transcript file line-by-line
- Extracts user/assistant messages with timestamps
- Parses content arrays (text, thinking blocks)
- Extracts tool calls: `Edit`, `Write`, `Read`, etc.
- Captures `file_path` from Edit/Write tool inputs

**History Loading** (`load_history`)
- Reads `~/.claude/history.jsonl` (stores user interaction history)
- Uses first user message (>5 chars, not starting with `/` or `<`) as session description
- Maps by sessionId

**Tasks Loading** (`load_tasks_by_session`)
- Scans `~/.claude/tasks/{sessionId}/` directories
- Reads `.json` files
- Extracts `id`, `subject` (as content), `status` fields
- Combines with agent todos for display

### UI Modules

#### `ui/mod.rs` - Layout & Rendering

**Main Layout**
```
┌─────────────────────────────────────────────────────────────┐
│         FULLSCREEN MODE (Enter from any panel)              │
│  ┌──────────────────────────────────────────────────────┐   │
│  │  Detail View (Chat or Diff) - full size             │   │
│  └──────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘

OR

┌──────────────────────┬──────────────────────────────────────┐
│  LEFT SIDEBAR (40%)  │  DETAIL PANE (60%)                  │
├──────────────────────┤                                      │
│  Sessions List       │  Chat View (default)                │
│                      │  - Formatted messages               │
│  ─── OR ──────────   │  - Tool calls shown inline          │
│  Files List          │  - Scrollable                       │
│  ─── OR ──────────   │  - Relative timestamps              │
│  Todos List          │                                      │
│                      │  OR                                 │
│                      │  Diff View (Files focused)          │
│                      │  - Syntax colored diff              │
│                      │  - Scroll by hunk (h/l keys)       │
│                      │  - Wraps long lines                 │
│                      │                                      │
│                      │  OR                                 │
│                      │  Embedded Terminal                  │
│                      │  - Live Claude session              │
│                      │  - Full keyboard/mouse pass-through │
└──────────────────────┴──────────────────────────────────────┘
│  Help Bar (1 line) - context-sensitive help                │
└──────────────────────────────────────────────────────────────┘
```

**Panel Visibility**
Left sidebar dynamically shows:
1. Always: Sessions list (divides height equally with other panels)
2. If current session has files: Files panel
3. If current session has todos: Todos panel

**Rendering Pipeline**
```rust
ui::draw(f, app) {
    if app.fullscreen {
        // Just show detail view
        draw_detail_view();
    } else {
        // Split: left (40%) + detail (60%)
        draw_left_panel();      // Sessions/Files/Todos
        draw_detail_view();     // Chat/Diff/Terminal
    }
    draw_help_bar();            // Context-sensitive help
    if app.show_help {
        draw_help_popup();      // Overlay help modal
    }
}
```

#### `ui/sessions.rs` - Session & Chat Rendering

**Session List** (`draw_session_list`)
- Shows status icon, session name (custom > description > project_name), activity time
- Selected session highlighted with blue background
- Truncates long names (max 25 chars)
- Below each session: "2m ago 45 msgs"
- Rename mode: input overlay when `app.renaming == true`

**Detail View** (`draw_detail_view`) - Router
- Detects what to show based on state:
  - `terminal_mode && embedded_terminal.is_some()` → PTY output
  - `diff_mode || Focus::Files` → Diff view
  - `Focus::Todos` → Todos preview
  - Default → Chat view

**Chat View** (`draw_messages`)
- Header: "Chat - {project_name}"
- Messages formatted with role icon + timestamp:
  ```
  ▶ You  14:23
    Your message here...
    └─ Read [completed]
    └─ Edit [completed]

  ◀ Claude  14:24
    Claude response...
  ```
- Tool calls shown as `└─ ToolName [status]`
- Word-wrapped at viewport width
- Scrollable: stored as `chat_scroll` offset
- Scrollbar on right edge shows position

**Diff View** (`draw_diff_view`)
- Shows file path as title
- Syntax coloring:
  - Green: additions (+)
  - Red: deletions (-)
  - Cyan: hunk headers (@@)
  - Yellow: diff metadata (diff/index lines)
  - Gray: context
- Long lines wrapped
- Scroll offset saved in `chat_scroll` (shared with chat)
- Jump to hunks: `h`/`l` keys find `@@` lines

**Embedded Terminal** (`draw_embedded_terminal`)
- Gets screen from vt100 parser
- Maps each cell (character, foreground, background, bold)
- Converts vt100 colors to ratatui colors
- Renders cursor position
- Resizes PTY on viewport changes

**Styling Helpers**
- `styled_block(title, is_active)` - Border styling (blue/green depending on focus)
- `relative_time(dt)` - Formats time delta ("<1m ago", "2h ago", etc.)
- `truncate(s, max_len)` - Unicode-safe truncation with "..." suffix
- `status_style(status)` - Color based on status string

### Terminal Module

#### `terminal.rs` - PTY & Terminal Emulation

**Architecture**
```
User Input (Keyboard)
    │
    └─ [Main Loop] ─ key_to_bytes() ─┐
                                      │
    ┌──────────────────────────────────┘
    │
    ▼
┌─────────────────────────────────────┐
│  EmbeddedTerminal (Master PTY Side) │
│  ┌─────────────────────────────────┐│
│  │ PTY Master                      ││
│  │ - Writes user input             ││
│  │ - Reads command output          ││
│  └─────────────────────────────────┘│
│              ▲      ▲                │
│              │      └─ [Reader Thread] ──┐
│              │      (spawned once)      │
│              │                          │
│  [write()] ──┘                          │
│                                        │
│  Arc<Mutex<vt100::Parser>>              │
│  - Stateful terminal emulator          │
│  - Processes VT100 escape sequences    │
│  - Maintains screen buffer & state     │
│  └─ get_screen_with_styles()           │
│     Returns cells: (char, fg, bg, bold)│
└─────────────────────────────────────────┘
    ▲
    └─ [Rendering] ─ UI renders each frame
                       polling get_screen_with_styles()

└──────── PTY Slave (Child Process) ────────┘
│                                           │
└─ bash -c "cd project && claude --resume"  │
   │                                        │
   └─ (Reads stdin, writes stdout/stderr)   │
```

**Key Components**

`EmbeddedTerminal::new(cols, rows)`
- Creates PTY pair via `portable_pty`
- Initializes vt100 parser for terminal emulation
- Returns writer box for sending input

`EmbeddedTerminal::spawn_claude(project_dir, session_id)`
- Executes: `bash -c "cd 'PROJECT' && claude --resume SESSION --dangerously-skip-permissions"`
- Spawns background reader thread:
  - Reads 4KB at a time from PTY master
  - Feeds each chunk to vt100 parser
  - Loops until EOF (process exits)
  - Locks parser behind Arc<Mutex<>>
- Drops child handle (lets process run detached)

`EmbeddedTerminal::get_screen_with_styles()`
- Locks parser and extracts screen state
- Returns 2D grid: row × col → (character, fg_color, bg_color, bold)
- Used each frame by UI renderer
- Converts vt100 color indices to ratatui colors

`EmbeddedTerminal::write(data)`
- Sends bytes to PTY master (user keystrokes)
- Flushes immediately

`EmbeddedTerminal::resize(cols, rows)`
- Resizes PTY (SIGWINCH)
- Updates parser screen size

**Threading Model**
- Main thread: event loop, UI rendering
- Reader thread per PTY session (spawned in `spawn_claude`):
  - Runs in background, detached
  - Reads PTY output, feeds parser
  - Exits when process closes or `running` flag set to false
- Arc<Mutex<>> used for thread-safe parser access

## Data Flow

### Session Discovery
```
APP START
  │
  ├─ load_data() (async)
  │   │
  │   ├─ ClaudeData::load()
  │   │   │
  │   │   ├─ load_sessions()
  │   │   │   └─ Scan ~/.claude/projects/*/
  │   │   │       Read .jsonl files → Session objects
  │   │   │
  │   │   ├─ load_agents()
  │   │   │   └─ Scan ~/.claude/todos/
  │   │   │       Read JSON files → Agent todos
  │   │   │
  │   │   ├─ load_history()
  │   │   │   └─ Parse history.jsonl
  │   │   │       Map first user messages to sessions
  │   │   │
  │   │   └─ load_tasks_by_session()
  │   │       └─ Scan ~/.claude/tasks/{sessionId}/
  │   │           Read task JSON files
  │   │
  │   ├─ Populate session.todos (merge agents + tasks)
  │   ├─ Populate session.description (from history)
  │   └─ Sort sessions by last_activity (newest first)
  │
  └─ App.sessions = ClaudeData.sessions
     App.agents = ClaudeData.agents
```

### Session Selection & Message Loading
```
USER SELECTS SESSION (j/k)
  │
  ├─ Update session_list_state.selected()
  │
  └─ EVENT LOOP DETECTS CHANGE
      │
      ├─ load_session_messages() (async)
      │   │
      │   ├─ Get session.file_path (points to .jsonl transcript)
      │   │
      │   ├─ Parse transcript line-by-line
      │   │   (JSONL format: one message per line)
      │   │
      │   ├─ Extract ChatMessage objects:
      │   │   - Role (user/assistant)
      │   │   - Content (text + thinking blocks)
      │   │   - Tool calls (Edit/Write/Read/etc.)
      │   │   - Timestamps
      │   │
      │   └─ Extract file paths from Edit/Write tool calls
      │
      ├─ get_file_changes(paths)
      │   │
      │   ├─ For each file:
      │   │   ├─ Run: git diff --numstat -- FILE
      │   │   │   (Get +additions -deletions)
      │   │   │
      │   │   └─ Run: git status --porcelain -- FILE
      │   │       (Get status: Modified/Added/Deleted/etc.)
      │   │
      │   └─ Return FileChange objects with git metadata
      │
      └─ App.current_messages = loaded messages
         App.current_file_changes = loaded changes
         App.selected_file_idx = 0 (reset to first)
         App.chat_scroll = 0 (reset to bottom)
```

### File Diff Display
```
USER SELECTS FILE (j/k in Files panel)
  │
  ├─ Update selected_file_idx
  │
  └─ load_file_diff() (async)
      │
      ├─ Run: git diff --color=never -- FILE_PATH
      │
      ├─ Parse diff into Line objects
      │   - Syntax color by line type (+/-/@@/etc.)
      │   - Wrap long lines to viewport width
      │
      └─ App.current_diff = formatted diff string
         Ready to render with scroll position
```

### Embedded Terminal Session
```
USER PRESSES 'o' ON SESSION
  │
  ├─ open_embedded_terminal()
  │   │
  │   ├─ Create EmbeddedTerminal (PTY setup)
  │   │
  │   ├─ spawn_claude(project_dir, session_id)
  │   │   │
  │   │   ├─ Start process: bash -c "cd PROJECT && claude --resume SESSION"
  │   │   │
  │   │   ├─ Spawn reader thread (watches PTY for output)
  │   │   │
  │   │   └─ Process runs in background
  │   │
  │   ├─ App.embedded_terminal = Some(terminal)
  │   ├─ App.terminal_mode = true
  │   └─ Focus = Detail
  │
  ├─ EVENT LOOP (terminal_mode == true)
  │   │
  │   ├─ Keyboard input
  │   │   └─ forward to terminal via send_to_terminal()
  │   │
  │   ├─ Each frame
  │   │   ├─ UI calls get_screen_with_styles()
  │   │   └─ Renders PTY output with color mapping
  │   │
  │   └─ Exit keys (Ctrl+q/Ctrl+\)
  │       └─ close_embedded_terminal()
  │
  └─ USER EXITS (Ctrl+q)
      │
      ├─ terminal.stop()
      │   └─ Set running flag = false
      │   └─ Send Ctrl+C to process
      │
      ├─ App.embedded_terminal = None
      ├─ App.terminal_mode = false
      └─ Back to normal view
```

## State Management

### Focus Navigation
The `Focus` enum controls which left panel is active:
- `Sessions` - navigate with j/k, select with Enter
- `Files` - navigate with j/k, filter with f, toggle tree with t
- `Todos` - navigate with j/k (scroll)
- `Detail` - view pane (right side)

**Focus Transitions**
```
         Tab/BackTab (toggle between left and detail)
            ↕
[Sessions] ←─→ [Detail]
    │
    └─ h/l: cycle to Files/Todos (if they have content)
         h: go back to Sessions
         l: go to Files (if exist) else Todos (if exist)

Focus.Detail can show:
  - Chat (default)
  - Diff (if diff_mode = true, or Files focused)
  - Terminal (if terminal_mode = true)
  - Todos (if Todos focused)
```

### Scroll State
Each panel maintains independent scroll position:
```
App.chat_scroll        - Detail pane (chat or diff)
App.chat_scroll_max    - Maximum scroll for detail pane
App.files_scroll       - Files list (unused, handled by index)
App.files_scroll_max   - Max (unused)
App.todos_scroll       - Todos list
App.todos_scroll_max   - Max todos scroll
```

**Scroll Direction Quirk**
- Chat view: scroll_up moves view upward (towards earlier messages)
- Diff view: scroll_up moves view downward (towards later lines)
  - Reason: diff "newer" content is further down in file
  - Kept consistent with vim-style diff navigation

### Reactive Updates
```
EVERY FRAME (terminal.draw):
  1. Auto-refresh all sessions (every 1 sec)
     └─ Re-scans ~/.claude/projects/
     └─ Updates message counts, status

  2. Detect session selection change
     └─ If changed: load_session_messages()
     └─ Extracts files and todos

  3. Render entire UI
     └─ Read-only access to App state
```

## Key Design Decisions

### Why Async/Await?
- File I/O in `load_data()`, `load_session_messages()` can be slow
- PTY reader runs in separate thread to avoid blocking main loop
- tokio runtime allows non-blocking waits without freezing UI

### Why Git Diff Inline?
- File changes are extracted from tool_calls in transcript
- Running `git diff` provides accurate status + line counts
- Avoids dependency on Claude's reported changes
- Live status reflects actual repository state

### Why vt100 Parser?
- Claude may output ANSI escape sequences (colors, bold, etc.)
- vt100 crate provides stateful terminal emulation
- Each cell stores color and style info
- Enables pixel-perfect rendering of terminal output

### Why PTY Not Socket?
- Claude CLI expects interactive terminal (TTY)
- PTY provides proper signal handling (Ctrl+C, etc.)
- Supports mouse input if needed in future
- portable_pty handles platform differences

### Why File Truncation Over Pagination?
- Screen real estate is limited in TUI
- Session names, file paths truncated to fit
- Ellipsis (...) indicates truncation
- Full info available in selection/header

### Why Relative Time?
- "2h ago" more useful than absolute timestamp
- Fits in narrow columns
- Quick visual scan for recently active sessions
- Refresh updates times automatically

### Why Vim Keybindings?
- hjkl navigation familiar to Vim users
- j/k for down/up universal in TUI apps
- h/l for horizontal navigation intuitive
- Combines with vim-style text movement (g/G for top/bottom)

### Why Double-Layer Left Sidebar?
- Sessions always visible (primary entry point)
- Files/Todos shown only when relevant
- Avoids empty panels
- Proportional sizing shares space fairly

## Performance Considerations

### Data Loading
- Session discovery: `O(n)` where n = number of projects
- Per session: one async file read for transcript
- Per transcript: line-by-line JSON parsing (lazy, not all loaded into memory)
- Refresh cycle: 1 second to avoid thrashing

### Rendering
- Each frame: UI reads entire App state (no partial updates)
- ratatui handles dirty region optimization
- PTY screen parsed once per frame (4KB buffer max)
- Diff parsing done on-demand when file selected

### Memory
- All sessions kept in memory (typically <100 in ~/.claude/projects)
- Current messages buffered (transcript usually <50KB per session)
- Diff not stored (parsed from git diff output each time)
- PTY parser maintains 1000-line history (configurable)

## Extending the Architecture

### Adding New Panels
1. Add variant to `Focus` enum
2. Add scroll state fields to `App`
3. Add draw function in `ui/`
4. Update `draw_left_panel` to conditionally show
5. Update key handlers in `events.rs`

### Adding New Commands
1. Add handler in `handle_key()` matching on KeyCode
2. Update App state or call async data loader
3. Add status message if needed
4. Update help text in `draw_help_bar()`

### Custom Data Sources
Replace `ClaudeData::load()` with alternative that populates same structures:
- Session discovery: implement custom scanner
- Message loading: parse different format
- Keep same data types → rest of UI works unchanged

### Terminal Size Limitations
Current code assumes:
- Min 80 cols, 24 rows for embedded terminal
- Truncation gracefully handles narrow terminals
- Responsive layout (40/60 split adjusts to screen size)

To test with small terminals:
```bash
LINES=20 COLUMNS=60 cargo run
```

## Testing the Architecture

### Load Sessions Without UI
```rust
#[tokio::main]
async fn main() {
    let data = ClaudeData::load().await.unwrap();
    println!("Sessions: {}", data.sessions.len());
}
```

### Test PTY Output
```rust
let mut term = EmbeddedTerminal::new(80, 24)?;
term.spawn_claude("/path/to/project", "session-id")?;
std::thread::sleep(Duration::from_secs(2));
if let Some(screen) = term.get_screen_with_styles() {
    println!("Screen rows: {}", screen.len());
}
```

### Manual UI Testing
```bash
# Start lazychat
cargo run

# Test session loading
j/k - navigate sessions

# Test file view
l - open files panel
j/k - select file
Enter - view full screen
Esc - back

# Test terminal mode
o - open Claude terminal
type commands, see output rendered in real-time
Ctrl+q - exit terminal
```

## Future Improvements

### Planned
- Fuzzy search across sessions/files
- Session export (save transcript to file)
- Custom session filtering/sorting
- Multi-line input for terminal commands
- Copy-to-clipboard for code blocks

### Architectural
- Replace immediate git calls with cached status
- Implement message pagination (load on-demand)
- Support multiple Claude instances simultaneously
- Plugin system for custom data sources

### Performance
- Lazy-load messages (load only visible range)
- Cache git diff results
- Implement efficient diff sync (only new changes)
- Parallel session discovery
