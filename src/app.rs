use crate::data::{claude::ClaudeData, Agent, ChatMessage, FileChange, FileStatus, Session};
use crate::terminal::EmbeddedTerminal;
use crate::config::presets::{Preset, PresetManager};
use crate::process::registry::ProcessRegistry;
use anyhow::Result;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    Presets,   // Left panel - preset selection
    Sessions,
    Todos,
    Files,
    Detail,
}

pub struct App {
    pub should_quit: bool,
    pub show_help: bool,

    // Status message (shows temporarily)
    pub status_message: Option<String>,
    pub status_is_error: bool,

    // Focus
    pub focus: Focus,

    // Data
    pub sessions: Vec<Session>,
    pub agents: Vec<Agent>,

    // Chat messages for selected session
    pub current_messages: Vec<ChatMessage>,
    pub messages_loading: bool,

    // Selection state
    pub session_list_state: ratatui::widgets::ListState,

    // Scroll state for chat view
    pub chat_scroll: u16,
    pub chat_scroll_max: u16,

    // Scroll state for todos panel
    pub todos_scroll: u16,
    pub todos_scroll_max: u16,

    // Scroll state for files panel
    pub files_scroll: u16,
    pub files_scroll_max: u16,

    // Edited files for current session (with git info)
    pub current_file_changes: Vec<FileChange>,
    pub selected_file_idx: usize,
    pub current_diff: String,
    pub diff_mode: bool,  // True when viewing diff in detail pane
    pub fullscreen: bool, // True when detail view is fullscreen

    // Rename input
    pub renaming: bool,
    pub rename_buffer: String,

    // File filter
    pub file_filter_active: bool,
    pub file_filter: String,
    pub file_tree_mode: bool, // Toggle between flat list and tree view

    // Embedded terminal for Claude sessions
    pub embedded_terminal: Option<EmbeddedTerminal>,
    pub terminal_mode: bool,
    pub editor_mode: bool, // True when terminal is running editor (vs claude)

    // Preset management (Phase 2)
    pub preset_manager: Option<PresetManager>,
    pub presets: Vec<Preset>,
    pub selected_preset_idx: usize,
    pub preset_filter: String,
    pub preset_filter_active: bool,

    // Process registry (Phase 1)
    pub process_registry: Option<ProcessRegistry>,
}

impl App {
    pub fn new() -> Self {
        let mut session_list_state = ratatui::widgets::ListState::default();
        session_list_state.select(Some(0));

        Self {
            should_quit: false,
            show_help: false,
            status_message: None,
            status_is_error: false,
            focus: Focus::Sessions,
            sessions: Vec::new(),
            agents: Vec::new(),
            current_messages: Vec::new(),
            messages_loading: false,
            session_list_state,
            chat_scroll: 0,
            chat_scroll_max: 0,
            todos_scroll: 0,
            todos_scroll_max: 0,
            files_scroll: 0,
            files_scroll_max: 0,
            current_file_changes: Vec::new(),
            selected_file_idx: 0,
            current_diff: String::new(),
            diff_mode: false,
            fullscreen: false,
            renaming: false,
            rename_buffer: String::new(),
            file_filter_active: false,
            file_filter: String::new(),
            file_tree_mode: true, // Default to tree view
            embedded_terminal: None,
            terminal_mode: false,
            editor_mode: false,

            // Preset management
            preset_manager: None,
            presets: Vec::new(),
            selected_preset_idx: 0,
            preset_filter: String::new(),
            preset_filter_active: false,

            // Process registry
            process_registry: None,
        }
    }

    pub async fn load_data(&mut self) -> Result<()> {
        let data = ClaudeData::load().await?;
        self.sessions = data.sessions;
        self.agents = data.agents;
        Ok(())
    }

    pub async fn load_session_messages(&mut self) -> Result<()> {
        if let Some(i) = self.session_list_state.selected() {
            if let Some(session) = self.sessions.get(i) {
                self.messages_loading = true;
                self.current_messages = ClaudeData::load_session_messages(session).await?;
                self.messages_loading = false;
                self.chat_scroll = 0;

                // Extract unique edited files from tool calls
                let mut file_paths: Vec<String> = self
                    .current_messages
                    .iter()
                    .flat_map(|m| &m.tool_calls)
                    .filter_map(|tc| tc.file_path.clone())
                    .collect();
                file_paths.sort();
                file_paths.dedup();

                // Get git diff info for each file
                self.current_file_changes = Self::get_file_changes(&file_paths).await;
                self.selected_file_idx = 0;
                self.current_diff = String::new();
                self.files_scroll = 0;
                self.todos_scroll = 0;

                // Reset diff mode when switching sessions - show chat view
                self.diff_mode = false;
            }
        }
        Ok(())
    }

    pub fn toggle_focus(&mut self) {
        match self.focus {
            Focus::Presets => self.focus = Focus::Detail,
            Focus::Sessions => self.focus = Focus::Detail,
            Focus::Todos => self.focus = Focus::Detail,
            Focus::Files => self.focus = Focus::Detail,
            Focus::Detail => {
                self.focus = Focus::Sessions;
                self.diff_mode = false;
            }
        }
    }

    pub fn selected_session_todos_count(&self) -> usize {
        self.selected_session().map(|s| s.todos.len()).unwrap_or(0)
    }

    pub fn todos_scroll_up(&mut self) {
        if self.todos_scroll > 0 {
            self.todos_scroll = self.todos_scroll.saturating_sub(1);
        }
    }

    pub fn todos_scroll_down(&mut self) {
        if self.todos_scroll < self.todos_scroll_max {
            self.todos_scroll += 1;
        }
    }

    pub fn toggle_help(&mut self) {
        self.show_help = !self.show_help;
    }

    pub fn start_rename(&mut self) {
        if let Some(session) = self.selected_session() {
            self.rename_buffer = session
                .custom_name
                .clone()
                .or_else(|| session.description.clone())
                .unwrap_or_default();
            self.renaming = true;
        }
    }

    pub fn cancel_rename(&mut self) {
        self.renaming = false;
        self.rename_buffer.clear();
    }

    pub fn confirm_rename(&mut self) {
        if let Some(i) = self.session_list_state.selected() {
            if let Some(session) = self.sessions.get_mut(i) {
                if self.rename_buffer.is_empty() {
                    session.custom_name = None;
                } else {
                    session.custom_name = Some(self.rename_buffer.clone());
                }
            }
        }
        self.renaming = false;
        self.rename_buffer.clear();
    }

    pub fn rename_input(&mut self, c: char) {
        self.rename_buffer.push(c);
    }

    pub fn rename_backspace(&mut self) {
        self.rename_buffer.pop();
    }

    pub fn start_file_filter(&mut self) {
        self.file_filter_active = true;
        self.file_filter.clear();
    }

    pub fn cancel_file_filter(&mut self) {
        self.file_filter_active = false;
        self.file_filter.clear();
    }

    pub fn file_filter_input(&mut self, c: char) {
        self.file_filter.push(c);
    }

    pub fn file_filter_backspace(&mut self) {
        self.file_filter.pop();
        if self.file_filter.is_empty() {
            self.file_filter_active = false;
        }
    }

    pub fn filtered_files(&self) -> Vec<&FileChange> {
        if self.file_filter.is_empty() {
            self.current_file_changes.iter().collect()
        } else {
            let filter_lower = self.file_filter.to_lowercase();
            self.current_file_changes
                .iter()
                .filter(|f| {
                    f.filename.to_lowercase().contains(&filter_lower)
                        || f.path.to_lowercase().contains(&filter_lower)
                })
                .collect()
        }
    }

    pub fn toggle_file_tree_mode(&mut self) {
        self.file_tree_mode = !self.file_tree_mode;
    }

    /// Get the full path of the currently selected file
    pub fn selected_file_path(&self) -> Option<&str> {
        self.current_file_changes
            .get(self.selected_file_idx)
            .map(|f| f.path.as_str())
    }

    /// Copy the selected file's full path to clipboard
    pub fn yank_file_path(&mut self) -> bool {
        if let Some(path) = self.selected_file_path() {
            use std::io::Write;
            use std::process::{Command, Stdio};

            // Use pbcopy on macOS
            if let Ok(mut child) = Command::new("pbcopy").stdin(Stdio::piped()).spawn() {
                if let Some(mut stdin) = child.stdin.take() {
                    if stdin.write_all(path.as_bytes()).is_ok() {
                        drop(stdin);
                        if child.wait().is_ok() {
                            return true;
                        }
                    }
                }
            }
        }
        false
    }

    pub fn set_status(&mut self, message: &str) {
        self.status_message = Some(message.to_string());
        self.status_is_error = false;
    }

    pub fn set_error(&mut self, message: &str) {
        self.status_message = Some(message.to_string());
        self.status_is_error = true;
    }

    pub fn clear_status(&mut self) {
        self.status_message = None;
    }

    pub fn list_next(&mut self) {
        let len = self.sessions.len();
        if len > 0 {
            let i = self.session_list_state.selected().unwrap_or(0);
            if i + 1 < len {
                self.session_list_state.select(Some(i + 1));
            }
        }
    }

    pub fn list_prev(&mut self) {
        let len = self.sessions.len();
        if len > 0 {
            let i = self.session_list_state.selected().unwrap_or(0);
            if i > 0 {
                self.session_list_state.select(Some(i - 1));
            }
        }
    }

    pub fn scroll_up(&mut self) {
        if self.chat_scroll < self.chat_scroll_max {
            self.chat_scroll = (self.chat_scroll + 3).min(self.chat_scroll_max);
        }
    }

    pub fn scroll_down(&mut self) {
        if self.chat_scroll > 0 {
            self.chat_scroll = self.chat_scroll.saturating_sub(3);
        }
    }

    pub fn scroll_top(&mut self) {
        self.chat_scroll = self.chat_scroll_max;
    }

    pub fn scroll_bottom(&mut self) {
        self.chat_scroll = 0;
    }

    pub fn open_embedded_terminal(&mut self, cols: u16, rows: u16) -> anyhow::Result<()> {
        if let Some(session) = self.selected_session().cloned() {
            let project_dir = if session.project.starts_with('/') {
                session.project.clone()
            } else {
                format!("/{}", session.project.replace('-', "/"))
            };

            let mut terminal = EmbeddedTerminal::new(cols, rows)?;
            terminal.spawn_claude(&project_dir, &session.id)?;
            self.embedded_terminal = Some(terminal);
            self.terminal_mode = true;
            self.focus = Focus::Detail;
        }
        Ok(())
    }

    pub fn open_new_embedded_terminal(&mut self, cols: u16, rows: u16) -> anyhow::Result<()> {
        let mut terminal = EmbeddedTerminal::new(cols, rows)?;
        terminal.spawn_new_claude()?;
        self.embedded_terminal = Some(terminal);
        self.terminal_mode = true;
        self.focus = Focus::Detail;
        Ok(())
    }

    pub fn open_editor(&mut self, cols: u16, rows: u16) -> anyhow::Result<()> {
        if self.current_file_changes.is_empty() {
            return Ok(());
        }

        // Get the currently selected file path
        let file_path = &self.current_file_changes[self.selected_file_idx].path;

        let mut terminal = EmbeddedTerminal::new(cols, rows)?;
        terminal.spawn_editor(file_path)?;
        self.embedded_terminal = Some(terminal);
        self.terminal_mode = true;
        self.editor_mode = true;
        self.focus = Focus::Detail;
        self.fullscreen = true;
        Ok(())
    }

    pub fn close_embedded_terminal(&mut self) {
        if let Some(ref mut term) = self.embedded_terminal {
            term.stop();
        }
        self.embedded_terminal = None;
        self.terminal_mode = false;

        // If we were in editor mode, return to diff view (not fullscreen)
        if self.editor_mode {
            self.editor_mode = false;
            self.fullscreen = false;
            self.diff_mode = true;
            self.focus = Focus::Files;
        }
    }

    pub fn send_to_terminal(&mut self, data: &[u8]) -> anyhow::Result<()> {
        if let Some(ref mut term) = self.embedded_terminal {
            term.write(data)?;
        }
        Ok(())
    }

    pub fn resize_terminal(&mut self, cols: u16, rows: u16) -> anyhow::Result<()> {
        if let Some(ref mut term) = self.embedded_terminal {
            term.resize(cols, rows)?;
        }
        Ok(())
    }

    /// Get selected session
    pub fn selected_session(&self) -> Option<&Session> {
        self.session_list_state
            .selected()
            .and_then(|i| self.sessions.get(i))
    }

    /// Get selected preset
    pub fn selected_preset(&self) -> Option<&Preset> {
        self.presets.get(self.selected_preset_idx)
    }

    /// Get git diff info for files
    async fn get_file_changes(file_paths: &[String]) -> Vec<FileChange> {
        let mut changes = Vec::new();

        for path in file_paths {
            let filename = std::path::Path::new(path)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(path)
                .to_string();

            // Try to get git diff stats for this file
            let (status, additions, deletions) = Self::get_git_stats(path).await;

            changes.push(FileChange {
                path: path.clone(),
                filename,
                status,
                additions,
                deletions,
            });
        }

        changes
    }

    async fn get_git_stats(file_path: &str) -> (FileStatus, u32, u32) {
        use tokio::process::Command;

        // Get diff stats
        let output = Command::new("git")
            .args(["diff", "--numstat", "--", file_path])
            .output()
            .await;

        if let Ok(output) = output {
            let stdout = String::from_utf8_lossy(&output.stdout);
            if let Some(line) = stdout.lines().next() {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    let additions = parts[0].parse().unwrap_or(0);
                    let deletions = parts[1].parse().unwrap_or(0);
                    return (FileStatus::Modified, additions, deletions);
                }
            }
        }

        // Check if file is untracked
        let status_output = Command::new("git")
            .args(["status", "--porcelain", "--", file_path])
            .output()
            .await;

        if let Ok(output) = status_output {
            let stdout = String::from_utf8_lossy(&output.stdout);
            if let Some(line) = stdout.lines().next() {
                let status_code = &line[..2];
                match status_code {
                    "??" => return (FileStatus::Untracked, 0, 0),
                    "A " | " A" => return (FileStatus::Added, 0, 0),
                    "D " | " D" => return (FileStatus::Deleted, 0, 0),
                    "R " => return (FileStatus::Renamed, 0, 0),
                    _ => {}
                }
            }
        }

        (FileStatus::Modified, 0, 0)
    }

    pub async fn load_file_diff(&mut self) {
        if let Some(file) = self.current_file_changes.get(self.selected_file_idx) {
            use tokio::process::Command;

            let output = Command::new("git")
                .args(["diff", "--color=never", "--", &file.path])
                .output()
                .await;

            if let Ok(output) = output {
                self.current_diff = String::from_utf8_lossy(&output.stdout).to_string();
                if self.current_diff.is_empty() {
                    // Maybe it's a new file, try to show content
                    if let Ok(content) = tokio::fs::read_to_string(&file.path).await {
                        self.current_diff = format!("New file: {}\n\n{}", file.path, content);
                    }
                }
            } else {
                self.current_diff = "Failed to load diff".to_string();
            }
        }
    }

    pub fn files_select_next(&mut self) {
        if !self.current_file_changes.is_empty()
            && self.selected_file_idx + 1 < self.current_file_changes.len()
        {
            self.selected_file_idx += 1;
        }
    }

    pub fn files_select_prev(&mut self) {
        if !self.current_file_changes.is_empty() && self.selected_file_idx > 0 {
            self.selected_file_idx -= 1;
        }
    }

    /// Jump to next diff hunk (@@)
    pub fn jump_to_next_hunk(&mut self) {
        let hunk_positions: Vec<usize> = self
            .current_diff
            .lines()
            .enumerate()
            .filter(|(_, line)| line.starts_with("@@"))
            .map(|(i, _)| i)
            .collect();

        if hunk_positions.is_empty() {
            return;
        }

        // Find next hunk after current scroll position (no wrap)
        let current_line = self.chat_scroll_max.saturating_sub(self.chat_scroll) as usize;
        for &pos in &hunk_positions {
            if pos > current_line {
                let new_scroll = self.chat_scroll_max.saturating_sub(pos as u16);
                self.chat_scroll = new_scroll;
                return;
            }
        }
        // At last hunk - don't wrap, stay at end
    }

    /// Jump to previous diff hunk (@@)
    pub fn jump_to_prev_hunk(&mut self) {
        let hunk_positions: Vec<usize> = self
            .current_diff
            .lines()
            .enumerate()
            .filter(|(_, line)| line.starts_with("@@"))
            .map(|(i, _)| i)
            .collect();

        if hunk_positions.is_empty() {
            return;
        }

        // Find prev hunk before current scroll position (no wrap)
        let current_line = self.chat_scroll_max.saturating_sub(self.chat_scroll) as usize;
        for &pos in hunk_positions.iter().rev() {
            if pos < current_line {
                let new_scroll = self.chat_scroll_max.saturating_sub(pos as u16);
                self.chat_scroll = new_scroll;
                return;
            }
        }
        // At first hunk - don't wrap, stay at beginning
    }

    pub fn load_presets(&mut self) -> Result<()> {
        match PresetManager::load() {
            Ok(pm) => {
                self.presets = pm.all().to_vec();
                self.preset_manager = Some(pm);
            }
            Err(e) => {
                self.set_error(&format!("Failed to load presets: {e}"));
            }
        }
        Ok(())
    }

    pub fn load_process_registry(&mut self) -> Result<()> {
        match ProcessRegistry::load() {
            Ok(reg) => {
                self.process_registry = Some(reg);
            }
            Err(e) => {
                self.set_error(&format!("Failed to load process registry: {e}"));
            }
        }
        Ok(())
    }

    pub fn preset_next(&mut self) {
        if !self.presets.is_empty() && self.selected_preset_idx + 1 < self.presets.len() {
            self.selected_preset_idx += 1;
        }
    }

    pub fn preset_prev(&mut self) {
        if self.selected_preset_idx > 0 {
            self.selected_preset_idx -= 1;
        }
    }
}
