# Claude Code Hooks Integration

Lazychat integrates with Claude Code's hooks system to provide real-time session status updates. This document explains how hooks work, how they're used by lazychat, and how to configure them for your setup.

## Table of Contents

- [What Are Claude Code Hooks?](#what-are-claude-code-hooks)
- [How Lazychat Uses Hooks](#how-lazychat-uses-hooks)
- [Status States](#status-states)
- [State File Format](#state-file-format)
- [Configuration](#configuration)
- [Hook Events](#hook-events)
- [Future Integrations](#future-integrations)
- [Troubleshooting](#troubleshooting)
- [Security Considerations](#security-considerations)

## What Are Claude Code Hooks?

Claude Code hooks are lifecycle events that trigger custom scripts or commands at specific points during a Claude Code session. They allow external tools (like lazychat) to monitor and respond to Claude's activities in real-time.

Hooks are defined in `~/.claude/settings.json` and execute shell commands when specific events occur:

- **PreToolUse**: Before Claude executes a tool (like Edit, Bash, Read)
- **PostToolUse**: After a tool completes
- **Notification**: When Claude sends a user notification
- **Stop**: When a session ends or is stopped

This enables tools like lazychat to track session state without constantly polling file modification times.

## How Lazychat Uses Hooks

Lazychat reads session status from two sources, in order of priority:

### 1. Hook-Written State Files (Primary)
When hooks are configured, Claude Code writes session state to:
```
~/.claude/session-state/{session_id}.state
```

These files contain a single-word state: `working`, `waiting`, or `idle`.

### 2. File Modification Time (Fallback)
If no state file exists, lazychat estimates status based on how recently the session file was modified:
- **working** (< 10 seconds) - Claude recently started processing
- **active** (< 2 minutes) - Recent activity detected
- **idle** (2-30 minutes) - Waiting for user input
- **inactive** (> 30 minutes) - No recent activity

The hook-based approach is more accurate because it captures the actual Claude state, while time-based detection can lag by several seconds.

## Status States

Lazychat displays session status with visual indicators:

| State | Indicator | Color | Meaning |
|-------|-----------|-------|---------|
| `working` | `▶` | Cyan | Claude is actively processing (tools, analysis, etc.) |
| `waiting` | `◆` | Magenta | Claude is waiting for user input |
| `idle` | `●` | Yellow | Session inactive, no recent updates |
| `inactive` | `○` | Gray | No activity for > 30 minutes |
| `active` | `▶` | Green | Recent activity (fallback detection only) |

The state file approach enables more precise "waiting" status detection than time-based methods alone.

## State File Format

### File Location
```
~/.claude/session-state/{session_id}.state
```

### Content Format
Single line containing one of these values:
```
working
waiting
idle
```

### Example
For a session with ID `abc123def456`:
```
~/.claude/session-state/abc123def456.state
```

Content:
```
working
```

### Reading State Files (Implementation Detail)

Lazychat reads state files in `src/data/claude.rs`:

```rust
// Check for state file first (written by Claude hooks)
let state_file = claude_dir.join("session-state").join(format!("{}.state", &session_id));
let status = if state_file.exists() {
    // Read state from hook-written file
    std::fs::read_to_string(&state_file)
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|_| "idle".to_string())
} else if let Some(mod_time) = &modified {
    // Fall back to time-based detection
    // ...
}
```

The implementation:
1. Checks if state file exists
2. Reads the file content and trims whitespace
3. Falls back to "idle" if the file can't be read
4. Falls back to time-based detection if no state file exists

## Configuration

### Complete Hooks Configuration

Add this to `~/.claude/settings.json`:

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

### Configuration Breakdown

#### PreToolUse Hook
**When it runs:** Before Claude executes any tool (Edit, Bash, Read, etc.)

**Action:** Sets session state to `working`

```bash
mkdir -p ~/.claude/session-state && \
jq -r '.session_id' | xargs -I{} sh -c 'echo working > ~/.claude/session-state/{}.state'
```

**Steps:**
1. Create `~/.claude/session-state` directory if it doesn't exist
2. Extract session ID from hook JSON input using `jq`
3. Write "working" to the session state file

#### Notification Hook
**When it runs:** When Claude sends a notification (typically when waiting for user input)

**Action:** Sets session state to `waiting`

```bash
jq -r '.session_id' | xargs -I{} sh -c 'echo waiting > ~/.claude/session-state/{}.state'
```

#### Stop Hook
**When it runs:** When the session ends or is stopped

**Action:** Deletes the state file to clean up

```bash
jq -r '.session_id' | xargs -I{} rm -f ~/.claude/session-state/{}.state
```

### JSON Input Schema

Claude Code provides hook context as JSON via stdin. The relevant fields for session-state tracking:

```json
{
  "session_id": "abc123def456",
  "timestamp": "2025-02-03T14:23:45Z",
  "event_type": "PreToolUse",
  "details": {
    "tool_name": "Edit",
    "file_path": "/path/to/file.rs"
  }
}
```

The `session_id` field uniquely identifies the Claude Code session.

## Hook Events

### Available Events

Claude Code provides the following hook events:

| Event | Triggers | Best Use Case |
|-------|----------|---------------|
| `PreToolUse` | Before any tool execution | Track when Claude starts work |
| `PostToolUse` | After tool completes | Track when Claude finishes work |
| `Notification` | When Claude needs user input | Track blocked/waiting state |
| `Stop` | When session ends | Cleanup, archive, notifications |

### Event Execution Order

Typical flow during a Claude session:

```
1. User sends message
2. PreToolUse hooks fire (if Claude uses tools)
3. Tool executes (Edit, Bash, etc.)
4. PostToolUse hooks fire
5. Claude sends notification (if waiting)
6. Notification hooks fire
7. User responds
8. Repeat from step 2
9. User stops session
10. Stop hooks fire
```

For lazychat, we primarily use:
- **PreToolUse** → Set to "working"
- **Notification** → Set to "waiting"
- **Stop** → Clean up state file

## File and Directory Structure

### Session State Directory
```
~/.claude/
├── session-state/
│   ├── abc123def456.state      # Session 1 status
│   ├── xyz789uvw012.state      # Session 2 status
│   └── ...
├── projects/
│   ├── path-to-project/
│   │   └── session-abc123.jsonl
│   └── ...
├── settings.json               # Hooks configured here
└── tasks/
    ├── abc123def456/
    │   ├── task-1.json
    │   └── task-2.json
    └── ...
```

### State File Lifecycle

1. **Created** - On first `PreToolUse` event (session starts working)
2. **Updated** - Whenever state changes (working → waiting → idle)
3. **Deleted** - On `Stop` event (session ends)
4. **Stale** - If not updated for >30 minutes (treated as "inactive")

## Future Integrations

### Planned Hook Features

#### 1. Real-Time Notifications
Use `Notification` events to trigger system notifications:

```json
{
  "hooks": {
    "Notification": [
      {
        "matcher": "",
        "hooks": [
          {
            "type": "command",
            "command": "osascript -e 'display notification \"Claude is waiting for input\" with title \"Claude Code\"'"
          }
        ]
      }
    ]
  }
}
```

#### 2. Auto-Refresh Triggers
Lazychat currently refreshes every 2 seconds. Future integration could trigger immediate refresh on state changes:

```json
{
  "hooks": {
    "PreToolUse": [
      {
        "matcher": "",
        "hooks": [
          {
            "type": "command",
            "command": "echo 'refresh' | nc localhost 9999"
          }
        ]
      }
    ]
  }
}
```

Lazychat could listen on a socket and refresh immediately instead of waiting for the timer.

#### 3. Session Activity Log
Archive hook events to a log file for historical analysis:

```json
{
  "hooks": {
    "PreToolUse": [
      {
        "matcher": "",
        "hooks": [
          {
            "type": "command",
            "command": "echo \"$(date -u +%Y-%m-%dT%H:%M:%SZ) PreToolUse\" >> ~/.claude/session-state/activity.log"
          }
        ]
      }
    ]
  }
}
```

#### 4. Multi-Agent Coordination
Hooks could notify other tools when specific agents are active:

```json
{
  "hooks": {
    "PreToolUse": [
      {
        "matcher": "architect|executor|researcher",
        "hooks": [
          {
            "type": "command",
            "command": "jq -r '.details.agent_type' | xargs -I{} sh -c 'echo {} > ~/.claude/active-agent.lock'"
          }
        ]
      }
    ]
  }
}
```

## Troubleshooting

### State Files Not Being Created

**Problem:** Status always shows as "active" or "inactive" (time-based fallback)

**Solutions:**

1. **Check hooks are configured**
   ```bash
   cat ~/.claude/settings.json | jq '.hooks'
   ```
   Should show PreToolUse, Notification, and Stop hooks configured.

2. **Verify session-state directory exists**
   ```bash
   ls -la ~/.claude/session-state/
   ```
   If missing:
   ```bash
   mkdir -p ~/.claude/session-state
   ```

3. **Test hook manually**
   ```bash
   echo '{"session_id":"test-session"}' | \
     jq -r '.session_id' | \
     xargs -I{} sh -c 'echo working > ~/.claude/session-state/{}.state'

   cat ~/.claude/session-state/test-session.state
   # Should output: working
   ```

4. **Check Claude Code is running**
   ```bash
   ps aux | grep claude
   ```

5. **Check settings.json syntax**
   ```bash
   jq . ~/.claude/settings.json
   ```
   Should validate without errors.

### State Files Not Updating

**Problem:** Status is stuck on one state (e.g., always "working")

**Solutions:**

1. **Check file permissions**
   ```bash
   ls -la ~/.claude/session-state/
   # Should be writable (755 or 777)
   chmod 755 ~/.claude/session-state
   ```

2. **Verify session ID matches**
   ```bash
   # Get active session IDs
   ls ~/.claude/projects/*/

   # Check state files
   ls ~/.claude/session-state/

   # They should match (with .jsonl vs .state extension)
   ```

3. **Check for stale files**
   ```bash
   find ~/.claude/session-state -mmin +120 -ls
   # Remove files older than 2 hours
   ```

4. **Enable debug logging (if implemented)**
   ```bash
   export LAZYCHAT_DEBUG=1
   lazychat
   ```

### Lazychat Not Detecting State Changes

**Problem:** Lazychat shows outdated status even though state files are being updated

**Solutions:**

1. **Increase refresh rate** (temporary)
   ```bash
   lazychat --refresh 1  # Refresh every 1 second instead of 2
   ```

2. **Check file read permissions**
   ```bash
   ls -la ~/.claude/session-state/
   # Should be readable by your user
   ```

3. **Restart lazychat**
   State is cached in memory; reload to see changes:
   ```bash
   # Kill lazychat
   q  # In the TUI
   # Or: pkill lazychat

   # Restart
   lazychat
   ```

4. **Check for race conditions**
   If hooks are very slow, state might not update in time:
   ```bash
   # Monitor state file changes
   watch -n 0.1 'cat ~/.claude/session-state/*.state 2>/dev/null | sort | uniq -c'
   ```

## Security Considerations

### Permission Model

State files are stored in `~/.claude/session-state/` with standard user permissions:

```
-rw-r--r-- 1 user group ~/.claude/session-state/session-id.state
```

**Security implications:**

- **User-readable**: Only your user can read session state
- **User-writable**: Only your user can modify state files
- **Not world-readable**: Other users on the system cannot see your sessions
- **Not executable**: Cannot be executed as scripts

### Information Leakage

Session state files contain minimal information:

```
working
```

**Data minimization:**
- Only includes status, not session content
- No file paths, arguments, or tool details
- No authentication tokens or credentials
- Session ID is the only identifier (randomly generated)

### Hook Injection

The hook configuration uses `jq` to extract session ID safely:

```bash
jq -r '.session_id' | xargs -I{} sh -c 'echo working > ~/.claude/session-state/{}.state'
```

**Security properties:**
- `jq -r` validates JSON input
- `xargs -I{}` prevents argument injection with proper quoting
- No shell metacharacters in the session ID string
- File permissions restrict to user only

### Recommendations

1. **Keep hooks simple**
   - Avoid executing untrusted code in hooks
   - Keep hook commands minimal and readable

2. **Review hooks regularly**
   ```bash
   cat ~/.claude/settings.json | jq '.hooks' | less
   ```

3. **Monitor state directory**
   ```bash
   # Check for unexpected changes
   find ~/.claude/session-state -type f -newer ~/.claude/settings.json -ls
   ```

4. **Secure sensitive shells**
   - If adding custom hooks with credentials, ensure they're not logged
   - Use `history -c` or similar to clear shell history if needed

5. **Validate JSON input**
   - Always use `jq` to parse Claude Code hook input
   - Never use `sed`, `awk`, or regex for JSON parsing
   - This prevents injection vulnerabilities

### Data Privacy

State files contain no sensitive information:
- No conversation content
- No file modifications
- No command execution details
- Only session status ("working", "waiting", "idle")

The actual conversation data stays in `~/.claude/projects/*/` and is not exposed by hooks.

## Examples

### Example 1: Basic Setup

Minimal configuration to enable real-time status:

```json
{
  "hooks": {
    "PreToolUse": [{
      "matcher": "",
      "hooks": [{
        "type": "command",
        "command": "mkdir -p ~/.claude/session-state && jq -r '.session_id' | xargs -I{} sh -c 'echo working > ~/.claude/session-state/{}.state'"
      }]
    }],
    "Stop": [{
      "matcher": "",
      "hooks": [{
        "type": "command",
        "command": "jq -r '.session_id' | xargs -I{} rm -f ~/.claude/session-state/{}.state"
      }]
    }]
  }
}
```

### Example 2: With Notifications

Add desktop notifications when waiting:

```json
{
  "hooks": {
    "PreToolUse": [{
      "matcher": "",
      "hooks": [{
        "type": "command",
        "command": "mkdir -p ~/.claude/session-state && jq -r '.session_id' | xargs -I{} sh -c 'echo working > ~/.claude/session-state/{}.state'"
      }]
    }],
    "Notification": [{
      "matcher": "",
      "hooks": [{
        "type": "command",
        "command": "jq -r '.session_id' | xargs -I{} sh -c 'echo waiting > ~/.claude/session-state/{}.state && osascript -e \"display notification \\\"Claude is waiting\\\" with title \\\"Claude Code\\\"\"'"
      }]
    }],
    "Stop": [{
      "matcher": "",
      "hooks": [{
        "type": "command",
        "command": "jq -r '.session_id' | xargs -I{} rm -f ~/.claude/session-state/{}.state"
      }]
    }]
  }
}
```

### Example 3: Activity Logging

Log all session state changes:

```json
{
  "hooks": {
    "PreToolUse": [{
      "matcher": "",
      "hooks": [
        {
          "type": "command",
          "command": "mkdir -p ~/.claude/session-state && jq -r '.session_id' | xargs -I{} sh -c 'echo working > ~/.claude/session-state/{}.state && echo \"$(date -u +%Y-%m-%dT%H:%M:%SZ) {} working\" >> ~/.claude/session-state/activity.log'"
        }
      ]
    }],
    "Notification": [{
      "matcher": "",
      "hooks": [{
        "type": "command",
        "command": "jq -r '.session_id' | xargs -I{} sh -c 'echo waiting > ~/.claude/session-state/{}.state && echo \"$(date -u +%Y-%m-%dT%H:%M:%SZ) {} waiting\" >> ~/.claude/session-state/activity.log'"
      }]
    }],
    "Stop": [{
      "matcher": "",
      "hooks": [{
        "type": "command",
        "command": "jq -r '.session_id' | xargs -I{} sh -c 'rm -f ~/.claude/session-state/{}.state && echo \"$(date -u +%Y-%m-%dT%H:%M:%SZ) {} stopped\" >> ~/.claude/session-state/activity.log'"
      }]
    }]
  }
}
```

## Reference

### Claude Code Hooks Documentation

For more information on Claude Code hooks, refer to:
- `~/.claude/CLAUDE.md` - User instructions for Claude Code configuration
- `~/.claude/settings.json` - Your hooks configuration file

### Lazychat Integration Points

Hooks are integrated in lazychat at:
- **Loading**: `src/data/claude.rs` (lines 357-378) - State file reading logic
- **Display**: `src/ui/sessions.rs` - Status indicator rendering
- **Configuration**: `README.md` - User-facing hooks documentation
- **Directory**: `~/.claude/session-state/` - State file storage

### Related Files

- Session transcript format: `~/.claude/projects/*/session-id.jsonl`
- Task definitions: `~/.claude/tasks/session-id/task-id.json`
- Session metadata: `~/.claude/history.jsonl`

---

**Last Updated:** February 3, 2025
**Maintained By:** Lazychat Contributors
**License:** MIT
