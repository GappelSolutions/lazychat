//! Headless terminal management for background Claude processes

use anyhow::{Context, Result};
use std::process::{Child, Command, Stdio};
use uuid::Uuid;

/// Validate a path doesn't contain traversal attacks
fn validate_path(path: &str) -> Result<()> {
    let path_buf = std::path::PathBuf::from(path);

    // Reject paths with ".." components
    for component in path_buf.components() {
        if matches!(component, std::path::Component::ParentDir) {
            anyhow::bail!("Path traversal not allowed: {}", path);
        }
    }

    Ok(())
}

/// Validate preset name only contains safe characters
fn validate_preset_name(name: &str) -> Result<()> {
    if !name
        .chars()
        .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
    {
        anyhow::bail!(
            "Invalid preset name (use alphanumeric, dash, underscore): {}",
            name
        );
    }
    Ok(())
}

/// A headless terminal instance running Claude
pub struct HeadlessTerminal {
    process: Child,
    session_id: String,
}

impl HeadlessTerminal {
    /// Spawn a new headless Claude instance
    pub fn spawn(cwd: &str, add_dirs: Vec<String>, extra_args: Vec<String>) -> Result<Self> {
        // Generate a unique session ID for this headless instance
        let session_id = Uuid::new_v4().to_string();

        // Validate inputs
        validate_path(cwd)?;
        for dir in &add_dirs {
            validate_path(dir)?;
        }

        // Build the claude command
        let mut cmd = Command::new("claude");

        // Set working directory
        cmd.current_dir(cwd);

        // Add additional directories if specified
        for dir in &add_dirs {
            cmd.arg("--add-dir").arg(dir);
        }

        // Add extra arguments from preset (e.g., --dangerously-skip-permissions)
        for arg in &extra_args {
            cmd.arg(arg);
        }

        // Set session ID for resumability
        cmd.arg("--session-id").arg(&session_id);

        // Run in headless mode (no TTY)
        cmd.stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());

        // Spawn the process
        let process = cmd
            .spawn()
            .context("Failed to spawn headless Claude process")?;

        Ok(Self {
            process,
            session_id,
        })
    }

    /// Get the session ID
    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    /// Get the process ID
    pub fn pid(&self) -> u32 {
        self.process.id()
    }

    /// Check if the process is still running
    pub fn is_alive(&mut self) -> bool {
        self.process
            .try_wait()
            .map(|s| s.is_none())
            .unwrap_or(false)
    }

    /// Terminate the headless instance
    pub fn terminate(mut self) -> Result<()> {
        self.process.kill()?;
        Ok(())
    }
}
