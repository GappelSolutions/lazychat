//! Process adoption - discover orphan Claude sessions

use anyhow::Result;
use std::collections::HashSet;
use std::fs;
use sysinfo::System;

/// An orphan Claude session found running but not managed by lazychat
#[derive(Debug, Clone)]
pub struct OrphanSession {
    pub session_id: String,
    pub pid: Option<u32>,
    pub cwd: Option<String>,
    pub status: String, // from state file: "working", "active", "idle"
}

/// Discover orphan Claude sessions that are not in the registry
pub fn discover_orphan_sessions(registered_pids: &HashSet<u32>) -> Result<Vec<OrphanSession>> {
    let mut orphans = Vec::new();

    let claude_dir = dirs::home_dir().unwrap_or_default().join(".claude");

    let state_dir = claude_dir.join("session-state");

    if !state_dir.exists() {
        return Ok(orphans);
    }

    // Get all running Claude processes
    let mut sys = System::new();
    sys.refresh_processes();

    let claude_processes: Vec<(u32, String)> = sys
        .processes()
        .iter()
        .filter_map(|(pid, proc)| {
            let name = proc.name();
            let cmd = proc.cmd().join(" ");

            // Check if this is a Claude process
            if name.contains("claude") || cmd.contains("claude") {
                Some((pid.as_u32(), cmd))
            } else {
                None
            }
        })
        .collect();

    // Read state files to find active sessions
    if let Ok(entries) = fs::read_dir(&state_dir) {
        for entry in entries.flatten() {
            let path = entry.path();

            if path.extension().and_then(|e| e.to_str()) != Some("state") {
                continue;
            }

            // Extract session ID from filename (e.g., "abc123.state")
            let session_id = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_string();

            if session_id.is_empty() {
                continue;
            }

            // Read status from state file
            let status = fs::read_to_string(&path)
                .map(|s| s.trim().to_string())
                .unwrap_or_else(|_| "unknown".to_string());

            // Only interested in active sessions
            if status != "working" && status != "active" && status != "idle" {
                continue;
            }

            // Try to find matching process
            let (pid, cwd) = find_process_for_session(&session_id, &claude_processes);

            // Skip if already registered
            if let Some(p) = pid {
                if registered_pids.contains(&p) {
                    continue;
                }
            }

            orphans.push(OrphanSession {
                session_id,
                pid,
                cwd,
                status,
            });
        }
    }

    Ok(orphans)
}

/// Try to find a running process for a session ID
fn find_process_for_session(
    session_id: &str,
    processes: &[(u32, String)],
) -> (Option<u32>, Option<String>) {
    for (pid, cmd) in processes {
        // Check if command contains --session-id or --resume with this session ID
        if cmd.contains(&format!("--session-id {session_id}"))
            || cmd.contains(&format!("--session-id={session_id}"))
            || cmd.contains(&format!("--resume {session_id}"))
            || cmd.contains(&format!("--resume={session_id}"))
        {
            // Try to extract cwd from the command
            let cwd = extract_cwd_from_cmd(cmd);
            return (Some(*pid), cwd);
        }
    }

    (None, None)
}

/// Extract working directory from command if possible
fn extract_cwd_from_cmd(cmd: &str) -> Option<String> {
    // Look for "cd '/path/to/dir'" pattern
    if let Some(start) = cmd.find("cd '") {
        let rest = &cmd[start + 4..];
        if let Some(end) = rest.find('\'') {
            return Some(rest[..end].to_string());
        }
    }

    // Look for "cd /path/to/dir" pattern (unquoted)
    if let Some(start) = cmd.find("cd /") {
        let rest = &cmd[start + 3..];
        let end = rest
            .find(|c: char| c.is_whitespace() || c == ';')
            .unwrap_or(rest.len());
        return Some(rest[..end].to_string());
    }

    None
}

/// Get all session IDs that have state files
pub fn get_active_session_ids() -> Result<Vec<String>> {
    let state_dir = dirs::home_dir()
        .unwrap_or_default()
        .join(".claude")
        .join("session-state");

    let mut sessions = Vec::new();

    if !state_dir.exists() {
        return Ok(sessions);
    }

    if let Ok(entries) = fs::read_dir(&state_dir) {
        for entry in entries.flatten() {
            let path = entry.path();

            if path.extension().and_then(|e| e.to_str()) == Some("state") {
                if let Some(session_id) = path.file_stem().and_then(|s| s.to_str()) {
                    sessions.push(session_id.to_string());
                }
            }
        }
    }

    Ok(sessions)
}
