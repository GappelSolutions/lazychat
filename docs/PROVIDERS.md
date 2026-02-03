# Provider Architecture Guide

This document describes how to add support for new AI coding assistants (providers) to lazychat. Currently, lazychat supports Claude Code. This guide provides a roadmap for extending it to support other tools like Cursor, GitHub Copilot, Aider, and similar AI coding assistants.

## Table of Contents

- [Overview](#overview)
- [Current Claude Implementation](#current-claude-implementation)
- [Proposed Provider Architecture](#proposed-provider-architecture)
- [Step-by-Step Guide](#step-by-step-guide)
- [Data Directory Structures](#data-directory-structures)
- [Terminal Spawning Patterns](#terminal-spawning-patterns)
- [Configuration](#configuration)
- [Example Implementation](#example-implementation)
- [Testing Your Provider](#testing-your-provider)

## Overview

Lazychat displays sessions, messages, todos, and files from AI coding assistants. Each assistant stores data differently and can be launched differently. The provider architecture abstracts these differences, allowing lazychat to support multiple assistants simultaneously.

### Key Concepts

**Provider**: An implementation that knows how to load data from and launch a specific AI tool.

**Session**: A conversation thread with the AI assistant.

**Data Directory**: Where the assistant stores sessions, history, and state (typically `~/.toolname/`).

**Terminal Spawning**: How to launch the assistant CLI within lazychat's embedded terminal.

## Current Claude Implementation

The current implementation is Claude Code-specific. Understanding it is the foundation for adding new providers.

### File Structure

```
src/data/
├── mod.rs          # Core data structures (Session, ChatMessage, etc.)
└── claude.rs       # Claude Code implementation
```

### Current Flow

1. **App startup** (`main.rs` → `app.rs::load_data()`)
   - Calls `ClaudeData::load()` directly
   - Loads sessions from `~/.claude/projects/`
   - Loads history from `~/.claude/history.jsonl`
   - Loads todos from `~/.claude/todos/`
   - Loads tasks from `~/.claude/tasks/`

2. **Session selection** (`app.rs::load_session_messages()`)
   - Reads session JSONL file
   - Parses user/assistant messages
   - Extracts tool calls (Edit, Write, Bash, etc.)
   - Determines file changes from tool calls

3. **Terminal spawning** (`terminal.rs::spawn_claude()`)
   - Creates PTY with `portable-pty`
   - Spawns: `bash -c "cd PROJECT_DIR && claude --resume SESSION_ID --dangerously-skip-permissions"`
   - Reads output with `vt100` parser
   - Displays rendered screen in TUI

### Key Files to Understand

**`src/data/claude.rs`**: 485 lines
- `ClaudeData::claude_dir()` - Gets `~/.claude` directory
- `ClaudeData::load_sessions()` - Reads from `~/.claude/projects/{project}/*.jsonl`
- `ClaudeData::load_session_messages()` - Parses JSONL format with tool calls
- `ClaudeData::load_history()` - Extracts first messages for descriptions
- `ClaudeData::load_tasks_by_session()` - Reads from `~/.claude/tasks/{sessionId}/`

**`src/data/mod.rs`**: Core structures
- `Session` - Metadata about a session
- `ChatMessage` - User/assistant messages
- `ToolCall` - Tool invocations (Edit, Write, Bash)
- `TodoItem` - Task items

**`src/terminal.rs`**: 170 lines
- `EmbeddedTerminal::spawn_claude()` - Hardcoded Claude spawn logic
- `EmbeddedTerminal::spawn_new_claude()` - New session spawn

## Proposed Provider Architecture

### Design Goals

1. **Abstraction** - Each provider implements a trait
2. **Parallel Support** - Multiple providers available simultaneously
3. **Lazy Loading** - Only load enabled providers
4. **Configuration** - User selects preferred provider(s)
5. **Backwards Compatible** - Claude remains the default

### Provider Trait

```rust
/// Trait for AI coding assistant data providers
pub trait AIProvider: Send + Sync {
    /// Provider identifier (e.g., "claude", "cursor", "copilot")
    fn id(&self) -> &str;

    /// Human-readable name
    fn name(&self) -> &str;

    /// Data directory path (e.g., ~/.claude, ~/.cursor)
    fn data_dir(&self) -> PathBuf;

    /// Check if this provider has any sessions
    async fn is_available(&self) -> bool;

    /// Load all sessions
    async fn load_sessions(&self) -> Result<Vec<Session>>;

    /// Load messages for a specific session
    async fn load_session_messages(&self, session: &Session) -> Result<Vec<ChatMessage>>;

    /// Load file changes (parse from messages/git)
    async fn load_file_changes(&self, session: &Session) -> Result<Vec<FileChange>>;

    /// Spawn the assistant in a terminal
    fn spawn_terminal(
        &self,
        terminal: &mut EmbeddedTerminal,
        project_dir: &str,
        session_id: &str,
    ) -> Result<()>;

    /// Spawn a new session
    fn spawn_new_terminal(&self, terminal: &mut EmbeddedTerminal) -> Result<()>;
}
```

### Provider Manager

```rust
/// Manages multiple AI providers
pub struct ProviderManager {
    providers: HashMap<String, Box<dyn AIProvider>>,
    active_provider: String,  // Default or user-selected
}

impl ProviderManager {
    pub fn new() -> Self {
        let mut providers = HashMap::new();

        // Register available providers
        providers.insert("claude".to_string(), Box::new(ClaudeProvider::new()));

        // Register additional providers if available
        if CursorProvider::is_installed() {
            providers.insert("cursor".to_string(), Box::new(CursorProvider::new()));
        }
        if CopilotProvider::is_installed() {
            providers.insert("copilot".to_string(), Box::new(CopilotProvider::new()));
        }

        // Set default to first available
        let active_provider = providers.keys().next().unwrap().clone();

        Self { providers, active_provider }
    }

    pub async fn load_all_sessions(&self) -> Result<Vec<Session>> {
        let mut all_sessions = Vec::new();
        for (_, provider) in &self.providers {
            if provider.is_available().await {
                let mut sessions = provider.load_sessions().await?;
                // Tag each session with its provider
                for session in &mut sessions {
                    session.provider = provider.id().to_string();
                }
                all_sessions.extend(sessions);
            }
        }
        Ok(all_sessions)
    }

    pub fn get_provider(&self, provider_id: &str) -> Option<&Box<dyn AIProvider>> {
        self.providers.get(provider_id)
    }
}
```

### Session Schema Enhancement

Add provider tracking to `Session`:

```rust
pub struct Session {
    pub id: String,
    pub provider: String,  // NEW: "claude", "cursor", "copilot"
    pub project: String,
    pub project_name: String,
    pub description: Option<String>,
    // ... rest unchanged
}
```

## Step-by-Step Guide

### 1. Understand the Tool's Data Storage

Before writing code, explore how the tool stores data:

```bash
# For your tool, find where sessions/history are stored
ls -la ~/.yourcli/
ls -la ~/.yourcli/projects/
cat ~/.yourcli/history.jsonl | head -20
```

**Questions to answer:**
- Where does it store sessions? (usually `~/.toolname/`)
- What file format? (JSONL, JSON, SQLite, etc.)
- How are messages structured?
- How are tool invocations tracked?
- Does it create state files?

### 2. Create Provider Implementation File

Create `src/data/your_provider.rs`:

```rust
use super::{AIProvider, Session, ChatMessage, ToolCall, FileChange, FileStatus};
use anyhow::Result;
use chrono::{DateTime, Utc};
use std::path::PathBuf;
use tokio::fs;

pub struct YourToolProvider;

impl YourToolProvider {
    pub fn new() -> Self {
        Self
    }

    fn data_dir() -> PathBuf {
        dirs::home_dir()
            .unwrap_or_default()
            .join(".yourtool")  // Change this
    }

    pub fn is_installed() -> bool {
        Self::data_dir().exists()
    }
}

#[async_trait::async_trait]
impl AIProvider for YourToolProvider {
    fn id(&self) -> &str {
        "yourtool"
    }

    fn name(&self) -> &str {
        "Your AI Tool"
    }

    fn data_dir(&self) -> PathBuf {
        Self::data_dir()
    }

    async fn is_available(&self) -> bool {
        Self::is_installed()
    }

    async fn load_sessions(&self) -> Result<Vec<Session>> {
        // Implementation here
        todo!()
    }

    async fn load_session_messages(&self, session: &Session) -> Result<Vec<ChatMessage>> {
        // Implementation here
        todo!()
    }

    async fn load_file_changes(&self, session: &Session) -> Result<Vec<FileChange>> {
        // Implementation here
        todo!()
    }

    fn spawn_terminal(
        &self,
        terminal: &mut EmbeddedTerminal,
        project_dir: &str,
        session_id: &str,
    ) -> Result<()> {
        // Implementation here
        todo!()
    }

    fn spawn_new_terminal(&self, terminal: &mut EmbeddedTerminal) -> Result<()> {
        // Implementation here
        todo!()
    }
}
```

### 3. Parse Session Metadata

```rust
async fn load_sessions(&self) -> Result<Vec<Session>> {
    let data_dir = Self::data_dir();
    let projects_dir = data_dir.join("projects");  // Adjust path
    let mut sessions = Vec::new();

    if !projects_dir.exists() {
        return Ok(sessions);
    }

    let mut dir_entries = fs::read_dir(&projects_dir).await?;

    while let Some(entry) = dir_entries.next_entry().await? {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let project_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        // Read session files (adapt to your tool's format)
        let mut project_entries = fs::read_dir(&path).await?;
        while let Some(file_entry) = project_entries.next_entry().await? {
            let file_path = file_entry.path();

            // Check file extension (JSONL, JSON, or your tool's format)
            if file_path.extension().and_then(|e| e.to_str()) != Some("jsonl") {
                continue;
            }

            let session_id = file_path
                .file_stem()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown")
                .to_string();

            let metadata = fs::metadata(&file_path).await?;
            let modified = metadata.modified().ok().map(DateTime::<Utc>::from);
            let file_size = metadata.len();

            sessions.push(Session {
                id: session_id,
                provider: self.id().to_string(),
                project: project_name.clone(),
                project_name: project_name.clone(),
                description: None,  // Load from history
                custom_name: None,
                started_at: modified,
                last_activity: modified,
                message_count: (file_size / 500).max(1),
                status: "idle".to_string(),
                todos: Vec::new(),
                file_path: Some(file_path),
            });
        }
    }

    sessions.sort_by(|a, b| b.last_activity.cmp(&a.last_activity));
    Ok(sessions)
}
```

### 4. Parse Chat Messages

Adapt parsing to your tool's message format:

```rust
async fn load_session_messages(&self, session: &Session) -> Result<Vec<ChatMessage>> {
    let file_path = match &session.file_path {
        Some(p) => p.clone(),
        None => return Ok(Vec::new()),
    };

    if !file_path.exists() {
        return Ok(Vec::new());
    }

    let content = fs::read_to_string(&file_path).await?;
    let mut messages = Vec::new();

    for line in content.lines() {
        if line.trim().is_empty() {
            continue;
        }

        if let Ok(json) = serde_json::from_str::<Value>(line) {
            let msg_type = json.get("type").and_then(|v| v.as_str()).unwrap_or("");

            match msg_type {
                "user" => {
                    // Parse user message - adapt to your format
                    if let Some(text) = json.get("message").and_then(|m| m.as_str()) {
                        messages.push(ChatMessage {
                            role: "user".to_string(),
                            content: text.to_string(),
                            timestamp: None,
                            tool_calls: Vec::new(),
                        });
                    }
                }
                "assistant" => {
                    // Parse assistant message with tool calls
                    let mut content = String::new();
                    let mut tool_calls = Vec::new();

                    if let Some(text) = json.get("message").and_then(|m| m.as_str()) {
                        content = text.to_string();
                    }

                    // Extract tool calls (adapt to your tool's format)
                    if let Some(tools) = json.get("tools").and_then(|t| t.as_array()) {
                        for tool in tools {
                            let tool_name = tool.get("name")
                                .and_then(|n| n.as_str())
                                .unwrap_or("unknown")
                                .to_string();

                            let file_path = tool.get("file_path")
                                .and_then(|p| p.as_str())
                                .map(|s| s.to_string());

                            tool_calls.push(ToolCall {
                                tool_name,
                                status: "completed".to_string(),
                                file_path,
                            });
                        }
                    }

                    if !content.is_empty() || !tool_calls.is_empty() {
                        messages.push(ChatMessage {
                            role: "assistant".to_string(),
                            content,
                            timestamp: None,
                            tool_calls,
                        });
                    }
                }
                _ => {}
            }
        }
    }

    Ok(messages)
}
```

### 5. Implement Terminal Spawning

```rust
fn spawn_terminal(
    &self,
    terminal: &mut EmbeddedTerminal,
    project_dir: &str,
    session_id: &str,
) -> Result<()> {
    use portable_pty::CommandBuilder;

    let mut cmd = CommandBuilder::new("bash");
    cmd.args([
        "-c",
        &format!(
            "cd '{}' 2>/dev/null || cd ~; yourtool --resume {} --option-flags",
            project_dir, session_id
        ),
    ]);

    terminal.spawn_command(cmd)?;
    Ok(())
}

fn spawn_new_terminal(&self, terminal: &mut EmbeddedTerminal) -> Result<()> {
    use portable_pty::CommandBuilder;

    let mut cmd = CommandBuilder::new("yourtool");
    // Add any default arguments

    terminal.spawn_command(cmd)?;
    Ok(())
}
```

### 6. Register Provider in Mod File

Update `src/data/mod.rs`:

```rust
pub mod claude;
pub mod cursor;  // NEW
pub mod your_provider;  // NEW

pub use claude::ClaudeProvider;
pub use cursor::CursorProvider;
pub use your_provider::YourToolProvider;
```

### 7. Update App to Use Provider Manager

Modify `src/app.rs`:

```rust
use crate::data::ProviderManager;

pub struct App {
    pub provider_manager: ProviderManager,
    // ... rest unchanged
}

impl App {
    pub fn new() -> Self {
        Self {
            provider_manager: ProviderManager::new(),
            // ... rest
        }
    }

    pub async fn load_data(&mut self) -> Result<()> {
        self.sessions = self.provider_manager.load_all_sessions().await?;
        Ok(())
    }
}
```

### 8. Update Terminal to Use Providers

Modify `src/app.rs::open_embedded_terminal()`:

```rust
pub fn open_embedded_terminal(&mut self, cols: u16, rows: u16) -> anyhow::Result<()> {
    if let Some(session) = self.selected_session().cloned() {
        let provider = self.provider_manager
            .get_provider(&session.provider)
            .ok_or_else(|| anyhow::anyhow!("Provider not found"))?;

        let project_dir = if session.project.starts_with('/') {
            session.project.clone()
        } else {
            format!("/{}", session.project.replace('-', "/"))
        };

        let mut terminal = EmbeddedTerminal::new(cols, rows)?;
        provider.spawn_terminal(&mut terminal, &project_dir, &session.id)?;
        self.embedded_terminal = Some(terminal);
        self.terminal_mode = true;
        self.focus = Focus::Detail;
    }
    Ok(())
}
```

## Data Directory Structures

### Claude Code (`~/.claude/`)

```
~/.claude/
├── projects/
│   ├── project-name/
│   │   ├── session-id-1.jsonl
│   │   └── session-id-2.jsonl
│   └── another-project/
│       └── session-id-3.jsonl
├── history.jsonl          # All messages (for descriptions)
├── tasks/
│   ├── session-id-1/
│   │   ├── task-1.json
│   │   └── task-2.json
│   └── session-id-2/
│       └── task-1.json
├── todos/                 # Legacy
│   ├── session-id-agent-agent-id-1.json
│   └── session-id-agent-agent-id-2.json
└── session-state/
    ├── session-id-1.state
    └── session-id-2.state
```

### Cursor (`~/.cursor/`)

Typical structure (verify with your Cursor installation):

```
~/.cursor/
├── projects/
│   └── {project-slug}/
│       └── {session-id}.jsonl
├── history.jsonl
└── sessions/
    ├── {session-id}.json
    └── {session-id}.state
```

### GitHub Copilot Chat (varies by editor)

VSCode integration:
```
~/.config/github-copilot/
├── conversations/
│   ├── {conversation-id}.json
│   └── {conversation-id}.md
└── chat-history.jsonl
```

Neovim/Other:
```
~/.copilot-chat/
├── sessions/
│   └── {session-id}.jsonl
└── history.jsonl
```

### Aider (`~/.aider/`)

```
~/.aider/
├── conversations/
│   ├── {uuid}.jsonl
│   └── {uuid}/
│       ├── files.txt
│       └── messages.jsonl
└── history.json
```

## Terminal Spawning Patterns

### Claude Code Pattern

```bash
cd PROJECT_DIR && claude --resume SESSION_ID --dangerously-skip-permissions
```

### Cursor Pattern

```bash
cd PROJECT_DIR && cursor --workspace SESSION_ID
```

### GitHub Copilot (VSCode)

```bash
code --new-window PROJECT_DIR
# (Copilot integrates directly; no CLI)
```

### Aider Pattern

```bash
aider --no-auto-commit PROJECT_DIR
# Or resume specific conversation
aider --resume {uuid}
```

### Generic Pattern

```rust
pub fn spawn_terminal(
    &self,
    terminal: &mut EmbeddedTerminal,
    project_dir: &str,
    session_id: &str,
) -> Result<()> {
    use portable_pty::CommandBuilder;

    let mut cmd = CommandBuilder::new("bash");
    let command = self.build_spawn_command(project_dir, session_id);
    cmd.args(["-c", &command]);

    terminal.spawn_command(cmd)?;
    Ok(())
}

fn build_spawn_command(&self, project_dir: &str, session_id: &str) -> String {
    // Tool-specific spawn logic
    format!(
        "cd '{}' 2>/dev/null || cd ~; {} {}",
        project_dir,
        self.cli_name(),
        self.spawn_args(session_id)
    )
}
```

## Configuration

### Proposed Config File

`~/.config/lazychat/config.toml`:

```toml
# Default provider (if multiple available)
default_provider = "claude"

# Enabled providers
[providers]
claude = { enabled = true, data_dir = "~/.claude" }
cursor = { enabled = true, data_dir = "~/.cursor" }
copilot = { enabled = false }
aider = { enabled = false, data_dir = "~/.aider" }

# UI
[ui]
theme = "dark"
show_hidden_sessions = false
sort_by = "last_activity"  # or "created", "name"

# Refresh
[watch]
interval_ms = 2000
```

### Loading Config

```rust
impl App {
    async fn load_config() -> Result<ProviderConfig> {
        let config_path = dirs::config_dir()
            .unwrap_or_default()
            .join("lazychat/config.toml");

        if !config_path.exists() {
            return Ok(ProviderConfig::default());
        }

        let content = tokio::fs::read_to_string(&config_path).await?;
        let config: ProviderConfig = toml::from_str(&content)?;
        Ok(config)
    }
}
```

## Example Implementation

### Complete Cursor Provider Skeleton

```rust
// src/data/cursor.rs

use super::{Session, ChatMessage, FileChange};
use anyhow::Result;
use chrono::{DateTime, Utc};
use serde_json::Value;
use std::path::PathBuf;
use tokio::fs;

pub struct CursorProvider;

impl CursorProvider {
    pub fn new() -> Self {
        Self
    }

    fn data_dir() -> PathBuf {
        dirs::home_dir()
            .unwrap_or_default()
            .join(".cursor")
    }

    pub fn is_installed() -> bool {
        Self::data_dir().exists()
    }

    /// Parse Cursor session format
    /// Cursor stores sessions similar to Claude but with different structure
    async fn parse_cursor_message(json: &Value) -> Option<(String, String, Vec<ToolCall>)> {
        let msg_type = json.get("role")?.as_str()?;
        let content = json.get("content")?.as_str()?.to_string();

        // Parse tool calls if present
        let mut tool_calls = Vec::new();
        if let Some(tools) = json.get("tool_calls").and_then(|t| t.as_array()) {
            for tool in tools {
                if let Some(name) = tool.get("function")
                    .and_then(|f| f.get("name"))
                    .and_then(|n| n.as_str()) {
                    tool_calls.push(ToolCall {
                        tool_name: name.to_string(),
                        status: "completed".to_string(),
                        file_path: None,
                    });
                }
            }
        }

        Some((msg_type.to_string(), content, tool_calls))
    }
}

#[async_trait::async_trait]
impl AIProvider for CursorProvider {
    fn id(&self) -> &str {
        "cursor"
    }

    fn name(&self) -> &str {
        "Cursor"
    }

    fn data_dir(&self) -> PathBuf {
        Self::data_dir()
    }

    async fn is_available(&self) -> bool {
        Self::is_installed()
    }

    async fn load_sessions(&self) -> Result<Vec<Session>> {
        let data_dir = Self::data_dir();
        let projects_dir = data_dir.join("projects");
        let mut sessions = Vec::new();

        if !projects_dir.exists() {
            return Ok(sessions);
        }

        // Similar to Claude implementation
        // Adapt to Cursor's actual structure
        Ok(sessions)
    }

    async fn load_session_messages(&self, session: &Session) -> Result<Vec<ChatMessage>> {
        let file_path = match &session.file_path {
            Some(p) => p.clone(),
            None => return Ok(Vec::new()),
        };

        if !file_path.exists() {
            return Ok(Vec::new());
        }

        let content = fs::read_to_string(&file_path).await?;
        let mut messages = Vec::new();

        for line in content.lines() {
            if line.trim().is_empty() {
                continue;
            }

            if let Ok(json) = serde_json::from_str::<Value>(line) {
                if let Some((role, content, tool_calls)) = Self::parse_cursor_message(&json).await {
                    messages.push(ChatMessage {
                        role,
                        content,
                        timestamp: None,
                        tool_calls,
                    });
                }
            }
        }

        Ok(messages)
    }

    async fn load_file_changes(&self, session: &Session) -> Result<Vec<FileChange>> {
        // Parse from messages or use git
        Ok(Vec::new())
    }

    fn spawn_terminal(
        &self,
        terminal: &mut EmbeddedTerminal,
        project_dir: &str,
        session_id: &str,
    ) -> Result<()> {
        use portable_pty::CommandBuilder;

        let mut cmd = CommandBuilder::new("bash");
        cmd.args([
            "-c",
            &format!(
                "cd '{}' 2>/dev/null || cd ~; cursor --workspace {}",
                project_dir, session_id
            ),
        ]);

        terminal.spawn_command(cmd)?;
        Ok(())
    }

    fn spawn_new_terminal(&self, terminal: &mut EmbeddedTerminal) -> Result<()> {
        use portable_pty::CommandBuilder;

        let mut cmd = CommandBuilder::new("cursor");
        terminal.spawn_command(cmd)?;
        Ok(())
    }
}
```

## Testing Your Provider

### 1. Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_is_installed() {
        let provider = YourToolProvider::new();
        // May be true or false depending on environment
        let _ = provider.is_available().await;
    }

    #[tokio::test]
    async fn test_load_sessions() {
        let provider = YourToolProvider::new();
        if provider.is_available().await {
            let sessions = provider.load_sessions().await;
            assert!(sessions.is_ok());
        }
    }

    #[tokio::test]
    async fn test_session_messages() {
        let provider = YourToolProvider::new();
        if provider.is_available().await {
            if let Ok(sessions) = provider.load_sessions().await {
                if let Some(session) = sessions.first() {
                    let messages = provider.load_session_messages(session).await;
                    assert!(messages.is_ok());
                }
            }
        }
    }
}
```

### 2. Manual Integration Testing

```bash
# Build with your new provider
cargo build --release

# Test loading sessions
./target/release/lazychat

# Check logs
RUST_LOG=debug ./target/release/lazychat

# Verify terminal spawning
# - Select a session with your provider
# - Press 'o' to open embedded terminal
# - Verify the tool launches correctly
```

### 3. Debugging

```rust
// Add debug logging
use tracing::{debug, info, error};

async fn load_sessions(&self) -> Result<Vec<Session>> {
    let data_dir = Self::data_dir();
    debug!("Loading sessions from: {:?}", data_dir);

    if !data_dir.exists() {
        error!("Data directory not found: {:?}", data_dir);
        return Ok(Vec::new());
    }

    // ... rest of implementation
    info!("Loaded {} sessions", sessions.len());
    Ok(sessions)
}
```

Enable debug output:
```bash
RUST_LOG=lazychat=debug ./target/release/lazychat
```

## Migration Checklist

When adding a new provider, ensure:

- [ ] Provider struct created with trait implementation
- [ ] Data directory path verified for the tool
- [ ] Session loading implemented and tested
- [ ] Message parsing handles tool calls
- [ ] File changes extracted correctly
- [ ] Terminal spawning works
- [ ] New session spawning works
- [ ] Provider registered in `mod.rs`
- [ ] Provider manager updated
- [ ] App uses provider manager
- [ ] Session struct includes provider field
- [ ] Terminal code uses provider
- [ ] Configuration system ready
- [ ] Documentation updated
- [ ] Unit tests passing
- [ ] Manual integration test passing

## Future Enhancements

1. **Provider Discovery** - Automatically detect installed tools
2. **Provider Settings** - Per-provider configuration options
3. **Session Import/Export** - Move sessions between providers
4. **Unified Search** - Search across all providers
5. **Provider Switching** - Easily switch providers in UI
6. **Custom Themes per Provider** - Provider-specific UI colors
7. **Plugin System** - Allow external provider implementations

## Resources

- [Cursor Documentation](https://cursor.sh/)
- [GitHub Copilot Chat](https://github.com/features/copilot/chat)
- [Aider Documentation](https://aider.chat/)
- [Claude Code Documentation](https://claude.ai/code)
- [Async Trait](https://docs.rs/async-trait/)
- [Portable PTY](https://docs.rs/portable-pty/)

## Contributing

To contribute a new provider:

1. Fork the repository
2. Create a feature branch: `git checkout -b feat/add-xyz-provider`
3. Implement the provider using this guide
4. Add tests
5. Update documentation
6. Submit a pull request

For questions, open an issue or check existing provider implementations.
