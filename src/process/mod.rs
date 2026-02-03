//! Process management for background Claude instances

pub mod adoption;
pub mod headless;
pub mod registry;

pub use adoption::{discover_orphan_sessions, OrphanSession};
pub use headless::HeadlessTerminal;
pub use registry::{ManagedProcess, ProcessRegistry};
