//! Process registry - tracks managed Claude processes

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// A Claude process managed by lazychat
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManagedProcess {
    pub pid: u32,
    pub session_id: String,
    pub preset_name: Option<String>,
    pub instance_index: u32,
    pub cwd: String,
    pub add_dirs: Vec<String>,
    pub started_at: DateTime<Utc>,
    pub status: String, // "running", "idle", "dead"
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct RegistryData {
    processes: Vec<ManagedProcess>,
}

/// Persistent registry for managed processes
pub struct ProcessRegistry {
    data: RegistryData,
    path: PathBuf,
}

impl ProcessRegistry {
    /// Load registry from ~/.cache/lazychat/processes.json
    pub fn load() -> Result<Self> {
        let path = Self::registry_path();

        // Ensure directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let data = if path.exists() {
            let content = fs::read_to_string(&path)?;
            match serde_json::from_str(&content) {
                Ok(d) => d,
                Err(e) => {
                    eprintln!("Warning: Corrupted process registry, resetting: {e}");
                    RegistryData::default()
                }
            }
        } else {
            RegistryData::default()
        };

        Ok(Self { data, path })
    }

    /// Get the registry file path
    fn registry_path() -> PathBuf {
        dirs::cache_dir()
            .unwrap_or_else(|| PathBuf::from("/tmp"))
            .join("lazychat")
            .join("processes.json")
    }

    /// Save registry to disk
    pub fn save(&self) -> Result<()> {
        let content = serde_json::to_string_pretty(&self.data)?;
        fs::write(&self.path, content)?;
        Ok(())
    }

    /// Register a new managed process
    pub fn register_process(
        &mut self,
        pid: u32,
        session_id: String,
        preset_name: Option<String>,
        instance_index: u32,
        cwd: String,
        add_dirs: Vec<String>,
    ) -> Result<()> {
        // Remove any existing entry with same PID
        self.data.processes.retain(|p| p.pid != pid);

        self.data.processes.push(ManagedProcess {
            pid,
            session_id,
            preset_name,
            instance_index,
            cwd,
            add_dirs,
            started_at: Utc::now(),
            status: "running".to_string(),
        });

        self.save()
    }

    /// Unregister a process by PID
    pub fn unregister_process(&mut self, pid: u32) -> Result<()> {
        self.data.processes.retain(|p| p.pid != pid);
        self.save()
    }

    /// Get all registered processes
    pub fn get_all_processes(&self) -> &[ManagedProcess] {
        &self.data.processes
    }

    /// Get mutable reference to all processes
    pub fn get_all_processes_mut(&mut self) -> &mut Vec<ManagedProcess> {
        &mut self.data.processes
    }

    /// Find process by PID
    pub fn find_by_pid(&self, pid: u32) -> Option<&ManagedProcess> {
        self.data.processes.iter().find(|p| p.pid == pid)
    }

    /// Find process by session ID
    pub fn find_by_session(&self, session_id: &str) -> Option<&ManagedProcess> {
        self.data
            .processes
            .iter()
            .find(|p| p.session_id == session_id)
    }

    /// Remove entries for PIDs that no longer exist
    pub fn cleanup_dead_processes(&mut self) -> Result<Vec<ManagedProcess>> {
        use sysinfo::{Pid, System};

        let mut sys = System::new();
        sys.refresh_processes();

        let mut dead = Vec::new();

        self.data.processes.retain(|p| {
            let pid = Pid::from_u32(p.pid);
            if sys.process(pid).is_some() {
                true
            } else {
                dead.push(p.clone());
                false
            }
        });

        if !dead.is_empty() {
            self.save()?;
        }

        Ok(dead)
    }

    /// Update status of a process
    pub fn update_status(&mut self, pid: u32, status: &str) -> Result<()> {
        if let Some(proc) = self.data.processes.iter_mut().find(|p| p.pid == pid) {
            proc.status = status.to_string();
            self.save()?;
        }
        Ok(())
    }
}
