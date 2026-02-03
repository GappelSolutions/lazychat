//! Lazychat library - TUI for AI coding assistants

pub mod config;
pub mod process;

// Re-export commonly used types
pub use config::{Preset, PresetManager};
pub use process::{discover_orphan_sessions, ManagedProcess, OrphanSession, ProcessRegistry};
