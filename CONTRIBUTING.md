# Contributing to lazychat

Thank you for your interest in contributing to lazychat! This document provides guidelines and instructions for all types of contributions.

## Table of Contents

- [Code of Conduct](#code-of-conduct)
- [Getting Started](#getting-started)
- [How to Report Bugs](#how-to-report-bugs)
- [How to Suggest Features](#how-to-suggest-features)
- [Development Setup](#development-setup)
- [Code Style](#code-style)
- [Making Changes](#making-changes)
- [Commit Message Conventions](#commit-message-conventions)
- [Pull Request Process](#pull-request-process)
- [Testing Guidelines](#testing-guidelines)
- [Project Structure](#project-structure)

---

## Code of Conduct

### Our Commitment

We are committed to providing a welcoming and inspiring community for all. We ask that you:

- **Be respectful** - Treat all community members with kindness and respect
- **Be inclusive** - Welcome and support people of all backgrounds
- **Assume good intent** - Interpret questions and comments charitably
- **Focus on impact** - Give feedback that is constructive and actionable
- **Report problems** - Use private channels to report Code of Conduct violations

### Expected Behavior

- Use inclusive language (avoid gendered terms, outdated references)
- Be professional in all interactions
- Welcome diverse perspectives and expertise
- Give credit where credit is due
- Help others succeed

### Unacceptable Behavior

- Harassment, discrimination, or exclusion based on any protected characteristic
- Unwelcome sexual advances or attention
- Deliberate intimidation or threats
- Spam or low-effort content
- Attacking ideas rather than engaging with substance

**Reporting**: If you witness or experience unacceptable behavior, please report it confidentially to the maintainers. We will investigate and take appropriate action.

---

## Getting Started

### Prerequisites

Before contributing, ensure you have:

- **Rust 1.70+** - Install from [rust-lang.org](https://www.rust-lang.org/tools/install)
- **Git** - For version control
- **A terminal with Unicode support** - For proper TUI rendering

### Fork & Clone

1. Fork the repository on GitHub
2. Clone your fork locally:
   ```bash
   git clone https://github.com/YOUR_USERNAME/lazychat.git
   cd lazychat
   ```
3. Add upstream remote:
   ```bash
   git remote add upstream https://github.com/GappelSolutions/lazychat.git
   ```

### Sync Before Starting

Always sync with the latest upstream before creating a branch:

```bash
git fetch upstream
git checkout main
git merge upstream/main
```

---

## How to Report Bugs

### Before Reporting

- Check existing [issues](https://github.com/GappelSolutions/lazychat/issues) to avoid duplicates
- Update to the latest version to confirm the bug still exists
- Try to reproduce the issue consistently
- Gather relevant information (OS, terminal, Rust version)

### Writing a Good Bug Report

Use the bug report template when creating an issue. Include:

**Title**: Clear, concise description of what's broken
```
Example: "Chat scrolling jumps to bottom when new messages arrive"
```

**Description**: What you were doing, what you expected, what happened instead

**Environment**:
```
- OS: macOS 12.6 / Linux (Ubuntu 22.04) / Windows 10
- Terminal: iTerm2 / GNOME Terminal / Windows Terminal
- Rust version: rustc 1.75.0
- lazychat version: 0.1.0 (or commit hash)
```

**Steps to Reproduce**:
1. Launch lazychat
2. Navigate to a session with many messages
3. Scroll up
4. New message arrives
5. Observe chat view jumps to bottom

**Expected Behavior**: Chat view maintains scroll position when new messages arrive

**Actual Behavior**: Chat view jumps to bottom, losing scroll context

**Screenshots/Logs**: Paste terminal output, error messages, or screenshots if helpful

### Example Issue

```markdown
## Title
Embedded terminal crashes when session directory contains spaces

## Description
When opening a session whose project path contains spaces, the embedded terminal fails to launch.

## Environment
- OS: macOS 13.2
- Terminal: iTerm2
- Rust: 1.75.0
- lazychat: main (7832889)

## Steps
1. Create a project directory with spaces: `mkdir "~/my project"`
2. Initialize a Claude session in it
3. Launch lazychat
4. Navigate to that session
5. Press 'o' to open embedded terminal
6. Application crashes

## Error
```
thread 'main' panicked at 'Failed to spawn PTY: Invalid path'
```

## Expected
Terminal should open, handling spaces in paths correctly
```

---

## How to Suggest Features

### Before Suggesting

- Check [issues](https://github.com/GappelSolutions/lazychat/issues) and [discussions](https://github.com/GappelSolutions/lazychat/discussions)
- Review the [README](README.md) and [Extending lazychat](README.md#extending-lazychat) section
- Understand the project scope (TUI for Claude Code sessions)

### Feature Request Format

**Title**: Clear, action-oriented description
```
Example: "Support filtering sessions by project name"
```

**Motivation**: Why this feature would be useful

**Proposed Solution**: How you imagine it working (including UI/UX sketches if helpful)

**Alternatives**: Other approaches you considered

**Example Use Case**: Real scenario where this helps

### Example Feature Request

```markdown
## Title
Add search functionality to filter messages in chat view

## Motivation
Sessions with 1000+ messages are hard to navigate. Currently, I have to manually scroll through to find a specific conversation. A search feature would make it much faster to find relevant context.

## Proposed Solution
- Add `Ctrl+f` keybinding to enter search mode in chat view
- Show a search box at the bottom: `/ search term`
- Highlight all matches in the chat
- `n`/`N` to jump to next/previous match
- `Esc` to exit search

## Alternatives
- Add a date/time filter instead (less specific)
- Add message type filter (Assistant/User/Tool) (complementary, not replacement)

## Use Case
I'm debugging authentication flow. Search for "password" would immediately show me all messages mentioning passwords, avoiding 5 minutes of scrolling through 2000 messages.
```

---

## Development Setup

### Build and Run

```bash
# Build in debug mode (faster compilation, slower runtime)
cargo build

# Run development build
cargo run

# Build optimized release (slower compilation, faster runtime)
cargo build --release

# Run release build
./target/release/lazychat
```

### Development Workflow

1. Make changes to source files
2. Rebuild with `cargo build`
3. Run `cargo run` to test
4. Check formatting: `cargo fmt --check`
5. Run clippy: `cargo clippy`
6. Run tests: `cargo test`

### Useful Commands

```bash
# Format code (auto-fixes most issues)
cargo fmt

# Check for style issues (warnings, best practices)
cargo clippy -- -D warnings

# Run tests with output
cargo test -- --nocapture

# Watch for changes and auto-rebuild
cargo watch -x build -x test

# Check dependencies for security issues
cargo audit

# Generate and view documentation
cargo doc --open
```

### Development Tips

- Use `cargo check` for quick syntax validation without building
- Set `RUST_LOG=debug` for detailed tracing output:
  ```bash
  RUST_LOG=debug cargo run
  ```
- Use `dbg!()` macro for debugging values
- Keep terminal window >80 columns wide for TUI development
- Test on different terminal emulators if possible

---

## Code Style

### Formatting

We use **rustfmt** for automatic code formatting. All contributions must pass formatting checks.

```bash
# Check formatting without modifying files
cargo fmt -- --check

# Auto-format all files
cargo fmt
```

**Format on save**: Configure your editor to run rustfmt on save:

**VSCode**: Add to `.vscode/settings.json`:
```json
{
  "[rust]": {
    "editor.formatOnSave": true,
    "editor.defaultFormatter": "rust-lang.rust-analyzer"
  }
}
```

**Neovim**: Use rust-analyzer with formatter integration

### Linting

We use **Clippy** to catch common mistakes and suggest idiomatic Rust. All contributions must pass Clippy checks with no warnings.

```bash
# Run clippy
cargo clippy -- -D warnings

# Fix some issues automatically
cargo clippy --fix
```

**Common Clippy warnings we address**:
- Simplify boolean comparisons
- Use `matches!()` instead of match with _ arms
- Avoid unnecessary references
- Use iterator methods instead of loops

### Naming Conventions

- **Functions & variables**: `snake_case`
  ```rust
  fn handle_key_press() { }
  let session_id = "abc123";
  ```

- **Types, structs, enums**: `PascalCase`
  ```rust
  pub struct SessionState { }
  pub enum FileStatus { }
  ```

- **Constants**: `UPPER_SNAKE_CASE`
  ```rust
  const DEFAULT_TIMEOUT: u64 = 30;
  const MAX_MESSAGES: usize = 10_000;
  ```

- **Modules**: `snake_case`
  ```rust
  mod data;
  mod ui;
  ```

- **Lifetimes**: `'a`, `'b`, etc. (lowercase)
  ```rust
  fn borrow<'a>(data: &'a str) { }
  ```

### Documentation

Write clear, idiomatic Rust documentation:

```rust
/// Brief one-line description ending with a period.
///
/// Longer explanation if the function is complex or non-obvious.
/// Include examples for public APIs.
///
/// # Arguments
///
/// * `session_id` - The unique identifier for the session
///
/// # Returns
///
/// A `Result` containing the session data or an error
///
/// # Errors
///
/// Returns an error if the session file cannot be read or parsed.
///
/// # Examples
///
/// ```
/// let session = load_session("session123")?;
/// println!("Session: {:?}", session);
/// ```
pub fn load_session(session_id: &str) -> Result<Session> {
    // implementation
}
```

### Code Comments

Use comments to explain **why**, not what:

```rust
// ✗ Bad - Explains what the code does (obvious from reading it)
i += 1;  // Increment i

// ✓ Good - Explains the intention and context
// Skip invalid sessions and only count active ones in results
if session.is_active() {
    count += 1;
}

// ✓ Great - Explains a non-obvious algorithm or workaround
// PTY requires absolute path; expand ~ to home directory
let expanded_path = if path.starts_with("~") {
    format!("{}{}", std::env::var("HOME")?, &path[1..])
} else {
    path.to_string()
};
```

### Error Handling

Use `Result<T>` and `?` operator; avoid `.unwrap()` in production code:

```rust
// ✗ Bad - Panics on error
let data = fs::read_to_string(path).unwrap();

// ✓ Good - Propagates error with context
let data = fs::read_to_string(path)
    .map_err(|e| format!("Failed to read config: {}", e))?;

// ✓ Also good - Using anyhow for context
let data = fs::read_to_string(path)
    .context("Failed to read config file")?;
```

### Import Organization

Organize imports in logical groups:

```rust
// Standard library
use std::io;
use std::path::Path;

// External crates (alphabetical)
use anyhow::Result;
use ratatui::prelude::*;
use tokio::sync::Mutex;

// Internal modules
use crate::app::App;
use crate::data::Session;
```

---

## Making Changes

### Create a Feature Branch

Always create a new branch for your changes:

```bash
git checkout main
git pull upstream main
git checkout -b feature/your-feature-name
```

**Branch naming conventions**:
- `feature/description` - New functionality
- `fix/description` - Bug fixes
- `docs/description` - Documentation updates
- `refactor/description` - Code improvements (no functional change)
- `test/description` - Adding or improving tests

### Keep Commits Logical

- Each commit should be atomic (one logical change)
- Don't mix unrelated changes
- Test after each commit: `cargo test`
- Use interactive rebase to clean up before PR:
  ```bash
  git rebase -i origin/main
  ```

---

## Commit Message Conventions

We follow a clear commit message format for readability and automated tooling.

### Format

```
<type>(<scope>): <subject>

<body>

<footer>
```

### Type

- `feat` - New feature
- `fix` - Bug fix
- `refactor` - Code refactoring (no functional change)
- `style` - Formatting, missing semicolons, etc.
- `test` - Adding or updating tests
- `docs` - Documentation updates
- `chore` - Build process, dependencies, tooling

### Scope

Scope is the part of the codebase affected:

- `ui` - User interface
- `data` - Data loading and structures
- `events` - Event handling
- `terminal` - Embedded terminal
- `config` - Configuration
- `none` - When not applicable

### Subject

- Use imperative mood: "add feature" not "added feature"
- Don't capitalize first letter (unless it's an acronym)
- No period at the end
- Limit to 50 characters

### Body

- Explain **what** and **why**, not how
- Wrap at 72 characters
- Leave a blank line between subject and body
- Use bullet points for multiple points
- Reference related issues: `Closes #123`, `Fixes #456`

### Footer

- Reference issues: `Closes #123`
- Breaking changes: `BREAKING CHANGE: description`

### Examples

**Good commit**:
```
feat(ui): add message search functionality

- Add Ctrl+f keybinding to enter search mode in chat view
- Highlight all matches with different colors
- Support n/N for next/previous match navigation
- Add search indicator to status bar

Closes #45
```

**Good fix**:
```
fix(terminal): handle spaces in session paths

The PTY spawner was failing when project paths contained spaces.
Use shellwords crate to properly escape path arguments.

Fixes #87
```

**Good refactor**:
```
refactor(data): extract message loading into separate module

No functional change. This prepares for multi-provider support
by isolating Claude-specific loading logic.
```

**Bad commits** (don't do this):
```
WIP                           # Too vague
Fix stuff                     # Doesn't explain what
Updated files                 # Passive voice
feat(ui): add search feat...  # Too long, truncated
Fix bugs and improve perf     # Multiple unrelated changes
```

---

## Pull Request Process

### Before Opening a PR

1. **Sync with upstream**:
   ```bash
   git fetch upstream
   git rebase upstream/main
   ```

2. **Run all checks**:
   ```bash
   cargo fmt
   cargo clippy -- -D warnings
   cargo test
   cargo build --release
   ```

3. **Update documentation** if needed:
   - README.md (if adding features)
   - In-code documentation (comments, doc comments)
   - Keybindings section (if adding keybindings)

4. **Add tests** for new functionality

### Opening a PR

1. Push to your fork: `git push origin feature/your-feature`
2. Go to GitHub and click "New Pull Request"
3. Fill in the PR template:

```markdown
## Description

Clear description of what this PR does. Link to related issues:
Closes #123

## Type of Change

- [ ] Bug fix (non-breaking change that fixes an issue)
- [ ] New feature (non-breaking change that adds functionality)
- [ ] Breaking change (fix or feature that would cause existing functionality to change)
- [ ] Documentation update

## Changes

- Item 1
- Item 2
- Item 3

## Testing Done

Describe how you tested these changes:
- [ ] Ran locally with `cargo run`
- [ ] Tested keybinding: ...
- [ ] Ran `cargo test`
- [ ] Tested on multiple terminals: ...

## Checklist

- [ ] My code follows the style guidelines of this project (`cargo fmt`, `cargo clippy`)
- [ ] I have performed a self-review of my code
- [ ] I have commented my code, particularly in hard-to-understand areas
- [ ] I have made corresponding changes to the documentation
- [ ] My changes generate no new warnings (`cargo clippy -- -D warnings`)
- [ ] I have added tests that prove my fix is effective or that my feature works
- [ ] New and existing unit tests pass locally with my changes
```

### PR Review Process

- Maintainers will review your code
- Be responsive to feedback
- Make requested changes in new commits (don't rebase existing commits after review starts)
- Once approved, maintainers will merge your PR

### PR Etiquette

- Keep PRs focused - one feature or fix per PR
- Make PRs early (work-in-progress is okay, mark as `[WIP]`)
- Respond to feedback within 72 hours (or let maintainers know if you're unavailable)
- Assume good intent when receiving feedback
- Don't take criticism of code as personal criticism

---

## Testing Guidelines

### Running Tests

```bash
# Run all tests
cargo test

# Run tests with output
cargo test -- --nocapture

# Run a specific test
cargo test test_load_session

# Run tests in parallel (default) or sequentially
cargo test -- --test-threads=1

# Run with all Clippy checks
cargo test && cargo clippy -- -D warnings
```

### Writing Tests

Write unit tests near the code they test:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_parsing() {
        let data = r#"{"id": "abc123", "messages": []}"#;
        let session = parse_session(data).unwrap();
        assert_eq!(session.id, "abc123");
        assert_eq!(session.messages.len(), 0);
    }

    #[test]
    fn test_empty_session_name() {
        let result = validate_session_name("");
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_async_loading() {
        let sessions = load_all_sessions().await.unwrap();
        assert!(!sessions.is_empty());
    }
}
```

**Test naming**: Use `test_<function>_<scenario>` format:
- `test_load_session_success`
- `test_load_session_file_not_found`
- `test_parse_message_with_tool_calls`

**Assertions**: Use specific assertions for clarity:

```rust
// ✗ Less clear
assert!(name.len() > 0);

// ✓ More clear
assert!(!name.is_empty());

// ✗ Generic
assert_eq!(status, "working");

// ✓ Specific
assert_eq!(session.status, SessionStatus::Working,
    "Expected working status, got: {:?}", session.status);
```

### Coverage

Aim for >80% test coverage on new code. Focus on:
- Happy paths
- Error cases
- Edge cases (empty inputs, very large inputs, special characters)
- Integration between modules

### Async Testing

Use `#[tokio::test]` for async functions:

```rust
#[tokio::test]
async fn test_concurrent_loading() {
    let s1 = load_session("1");
    let s2 = load_session("2");
    let (r1, r2) = tokio::join!(s1, s2);
    assert!(r1.is_ok());
    assert!(r2.is_ok());
}
```

---

## Project Structure

Understanding the codebase organization helps you make better contributions:

```
lazychat/
├── src/
│   ├── main.rs              # Entry point, CLI args, terminal setup
│   ├── app.rs               # Central App state and logic
│   ├── events.rs            # Keyboard/mouse event handling
│   ├── terminal.rs          # Embedded PTY terminal
│   ├── data/
│   │   ├── mod.rs           # Data structures (Session, Message, etc.)
│   │   └── claude.rs        # Claude Code data loader
│   └── ui/
│       ├── mod.rs           # Main UI layout and rendering
│       └── sessions.rs      # Session-specific UI components
├── Cargo.toml               # Dependencies and project metadata
├── Cargo.lock               # Locked dependency versions
├── README.md                # User-facing documentation
├── CONTRIBUTING.md          # This file
└── LICENSE                  # MIT License
```

### Module Purposes

| Module | Purpose | Key Types/Functions |
|--------|---------|---------------------|
| `main.rs` | Terminal initialization and event loop | `main()`, `Args` |
| `app.rs` | Application state and logic | `App`, `Focus` enum |
| `events.rs` | Keyboard input handling | `run_app()`, `handle_key()` |
| `terminal.rs` | PTY wrapper for embedded terminal | `EmbeddedTerminal` |
| `data/mod.rs` | Data structures | `Session`, `ChatMessage`, `FileChange` |
| `data/claude.rs` | Loading Claude sessions/messages | `ClaudeData` |
| `ui/mod.rs` | Rendering all UI panels | `draw_ui()`, `draw_*()` functions |
| `ui/sessions.rs` | Session list and detail views | Session-specific rendering |

### Adding New Features

1. **Add data structures** to `src/data/mod.rs`
2. **Add loading logic** to `src/data/claude.rs`
3. **Add state** to `App` struct in `src/app.rs`
4. **Add rendering** to `src/ui/mod.rs` or new module
5. **Add event handling** to `src/events.rs`
6. **Add tests** to the relevant module
7. **Update README.md** if user-facing

---

## Common Tasks

### Adding a New Keybinding

1. Edit `src/events.rs`
2. Add case to `handle_key()` function
3. Implement the action
4. Update README.md keybindings table
5. Update help text in UI

### Adding a New Panel

1. Add variant to `Focus` enum in `src/app.rs`
2. Add state fields to `App` struct
3. Add draw function to `src/ui/mod.rs`
4. Add event handling in `src/events.rs`
5. Add keybindings to switch to new panel

### Fixing a Bug

1. Create test that reproduces bug
2. Verify test fails
3. Fix the bug
4. Verify test passes
5. Run full test suite

### Performance Improvements

1. Profile with `flamegraph` or `perf`
2. Create benchmark before optimization
3. Make change
4. Verify benchmark improves
5. Document in commit message why this helps

---

## Getting Help

- **Questions about contribution process**: Open a discussion
- **Help with Rust**: Check [The Book](https://doc.rust-lang.org/book/), [Rust by Example](https://doc.rust-lang.org/rust-by-example/)
- **TUI development**: See [ratatui docs](https://docs.rs/ratatui/latest/ratatui/)
- **Stuck on a bug**: Ask in discussions or comment on related issues
- **Async/Tokio help**: See [Tokio tutorial](https://tokio.rs/tokio/tutorial)

---

## Recognition

Contributors will be recognized in:
- Pull request comments
- CHANGELOG.md (major contributions)
- Contributors section of README.md (recurring contributors)

Thank you for contributing to lazychat!
