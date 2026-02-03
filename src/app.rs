use crate::data::{claude::ClaudeData, Session, Task, Agent, DailyStats, ChatMessage};
use crate::terminal::EmbeddedTerminal;
use anyhow::Result;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Sessions,
    Dashboard,
    Agents,
    Tasks,
    Stats,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
    Normal,
    Input,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    List,
    Detail,
}

impl Tab {
    pub fn all() -> Vec<Tab> {
        vec![Tab::Sessions, Tab::Dashboard, Tab::Agents, Tab::Tasks, Tab::Stats]
    }

    pub fn title(&self) -> &'static str {
        match self {
            Tab::Sessions => "Sessions",
            Tab::Dashboard => "Dashboard",
            Tab::Agents => "Agents",
            Tab::Tasks => "Tasks",
            Tab::Stats => "Stats",
        }
    }

    pub fn next(&self) -> Tab {
        match self {
            Tab::Sessions => Tab::Dashboard,
            Tab::Dashboard => Tab::Agents,
            Tab::Agents => Tab::Tasks,
            Tab::Tasks => Tab::Stats,
            Tab::Stats => Tab::Sessions,
        }
    }

    pub fn prev(&self) -> Tab {
        match self {
            Tab::Sessions => Tab::Stats,
            Tab::Dashboard => Tab::Sessions,
            Tab::Agents => Tab::Dashboard,
            Tab::Tasks => Tab::Agents,
            Tab::Stats => Tab::Tasks,
        }
    }
}

pub struct App {
    pub should_quit: bool,
    pub current_tab: Tab,
    pub refresh_rate: u64,
    pub show_help: bool,

    // Status message (shows temporarily)
    pub status_message: Option<String>,
    pub status_is_error: bool,

    // Input
    pub input_mode: InputMode,
    pub input_buffer: String,
    pub focus: Focus,

    // Data
    pub sessions: Vec<Session>,
    pub agents: Vec<Agent>,
    pub tasks: Vec<Task>,
    pub daily_stats: Vec<DailyStats>,

    // Chat messages for selected session
    pub current_messages: Vec<ChatMessage>,
    pub messages_loading: bool,

    // Selection state
    pub session_list_state: ratatui::widgets::ListState,
    pub agent_list_state: ratatui::widgets::ListState,
    pub task_list_state: ratatui::widgets::ListState,

    // Scroll state for chat view
    pub chat_scroll: u16,
    pub chat_scroll_max: u16,

    // Diff view state
    pub show_diff_view: bool,
    pub diff_inline_view: bool,  // Full-screen single file diff
    pub diff_scroll: u16,
    pub diff_scroll_max: u16,
    pub selected_diff_idx: usize,

    // Embedded terminal for Claude sessions
    pub embedded_terminal: Option<EmbeddedTerminal>,
    pub terminal_mode: bool,

    // Expanded agent (for tree view)
    pub expanded_agents: std::collections::HashSet<String>,
}

impl App {
    pub fn new(refresh_rate: u64) -> Self {
        let mut session_list_state = ratatui::widgets::ListState::default();
        session_list_state.select(Some(0));

        let mut agent_list_state = ratatui::widgets::ListState::default();
        agent_list_state.select(Some(0));

        let mut task_list_state = ratatui::widgets::ListState::default();
        task_list_state.select(Some(0));

        Self {
            should_quit: false,
            current_tab: Tab::Sessions,
            refresh_rate,
            show_help: false,
            status_message: None,
            status_is_error: false,
            input_mode: InputMode::Normal,
            input_buffer: String::new(),
            focus: Focus::List,
            sessions: Vec::new(),
            agents: Vec::new(),
            tasks: Vec::new(),
            daily_stats: Vec::new(),
            current_messages: Vec::new(),
            messages_loading: false,
            session_list_state,
            agent_list_state,
            task_list_state,
            chat_scroll: 0,
            chat_scroll_max: 0,
            show_diff_view: false,
            diff_inline_view: false,
            diff_scroll: 0,
            diff_scroll_max: 0,
            selected_diff_idx: 0,
            embedded_terminal: None,
            terminal_mode: false,
            expanded_agents: std::collections::HashSet::new(),
        }
    }

    pub async fn load_data(&mut self) -> Result<()> {
        let data = ClaudeData::load().await?;
        self.sessions = data.sessions;
        self.agents = data.agents;
        self.tasks = data.tasks;
        self.daily_stats = data.daily_stats;
        Ok(())
    }

    pub async fn load_session_messages(&mut self) -> Result<()> {
        if let Some(i) = self.session_list_state.selected() {
            if let Some(session) = self.sessions.get(i) {
                self.messages_loading = true;
                self.current_messages = ClaudeData::load_session_messages(session).await?;
                self.messages_loading = false;
                // Scroll to bottom
                self.chat_scroll = 0;
            }
        }
        Ok(())
    }

    pub fn next_tab(&mut self) {
        self.current_tab = self.current_tab.next();
        self.focus = Focus::List;
    }

    pub fn prev_tab(&mut self) {
        self.current_tab = self.current_tab.prev();
        self.focus = Focus::List;
    }

    pub fn select_tab(&mut self, index: usize) {
        let tabs = Tab::all();
        if index < tabs.len() {
            self.current_tab = tabs[index];
            self.focus = Focus::List;
        }
    }

    pub fn toggle_focus(&mut self) {
        self.focus = match self.focus {
            Focus::List => Focus::Detail,
            Focus::Detail => Focus::List,
        };
    }

    pub fn toggle_help(&mut self) {
        self.show_help = !self.show_help;
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

    pub fn enter_input_mode(&mut self) {
        self.input_mode = InputMode::Input;
    }

    pub fn exit_input_mode(&mut self) {
        self.input_mode = InputMode::Normal;
        self.input_buffer.clear();
    }

    pub fn input_char(&mut self, c: char) {
        self.input_buffer.push(c);
    }

    pub fn input_backspace(&mut self) {
        self.input_buffer.pop();
    }

    pub fn submit_input(&mut self) -> Option<String> {
        if self.input_buffer.is_empty() {
            return None;
        }
        let input = self.input_buffer.clone();
        self.input_buffer.clear();
        self.input_mode = InputMode::Normal;
        Some(input)
    }

    pub fn list_next(&mut self) {
        match self.current_tab {
            Tab::Sessions => {
                let len = self.sessions.len();
                if len > 0 {
                    let i = self.session_list_state.selected().unwrap_or(0);
                    self.session_list_state.select(Some((i + 1) % len));
                }
            }
            Tab::Agents => {
                let len = self.agents.len();
                if len > 0 {
                    let i = self.agent_list_state.selected().unwrap_or(0);
                    self.agent_list_state.select(Some((i + 1) % len));
                }
            }
            Tab::Tasks => {
                let len = self.tasks.len();
                if len > 0 {
                    let i = self.task_list_state.selected().unwrap_or(0);
                    self.task_list_state.select(Some((i + 1) % len));
                }
            }
            _ => {}
        }
    }

    pub fn list_prev(&mut self) {
        match self.current_tab {
            Tab::Sessions => {
                let len = self.sessions.len();
                if len > 0 {
                    let i = self.session_list_state.selected().unwrap_or(0);
                    self.session_list_state.select(Some(if i == 0 { len - 1 } else { i - 1 }));
                }
            }
            Tab::Agents => {
                let len = self.agents.len();
                if len > 0 {
                    let i = self.agent_list_state.selected().unwrap_or(0);
                    self.agent_list_state.select(Some(if i == 0 { len - 1 } else { i - 1 }));
                }
            }
            Tab::Tasks => {
                let len = self.tasks.len();
                if len > 0 {
                    let i = self.task_list_state.selected().unwrap_or(0);
                    self.task_list_state.select(Some(if i == 0 { len - 1 } else { i - 1 }));
                }
            }
            _ => {}
        }
    }

    // k - scroll up to see OLDER messages (increase scroll offset from bottom)
    pub fn scroll_up(&mut self) {
        if self.chat_scroll < self.chat_scroll_max {
            self.chat_scroll = (self.chat_scroll + 3).min(self.chat_scroll_max);
        }
    }

    // j - scroll down to see NEWER messages (decrease scroll offset, towards bottom)
    pub fn scroll_down(&mut self) {
        if self.chat_scroll > 0 {
            self.chat_scroll = self.chat_scroll.saturating_sub(3);
        }
    }

    // g - go to TOP (oldest messages)
    pub fn scroll_top(&mut self) {
        self.chat_scroll = self.chat_scroll_max;
    }

    // G - go to BOTTOM (newest/latest messages)
    pub fn scroll_bottom(&mut self) {
        self.chat_scroll = 0;
    }

    pub fn toggle_diff_view(&mut self) {
        self.show_diff_view = !self.show_diff_view;
        self.diff_inline_view = false;
        self.diff_scroll = 0;
        self.selected_diff_idx = 0;
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

    pub fn close_embedded_terminal(&mut self) {
        if let Some(ref mut term) = self.embedded_terminal {
            term.stop();
        }
        self.embedded_terminal = None;
        self.terminal_mode = false;
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

    pub fn get_all_file_changes(&self) -> Vec<&crate::data::FileChange> {
        self.current_messages
            .iter()
            .flat_map(|m| m.file_changes.iter())
            .collect()
    }

    pub fn diff_next(&mut self) {
        let changes = self.get_all_file_changes();
        if !changes.is_empty() {
            self.selected_diff_idx = (self.selected_diff_idx + 1) % changes.len();
            self.diff_scroll = 0;
        }
    }

    pub fn diff_prev(&mut self) {
        let changes = self.get_all_file_changes();
        if !changes.is_empty() {
            self.selected_diff_idx = if self.selected_diff_idx == 0 {
                changes.len() - 1
            } else {
                self.selected_diff_idx - 1
            };
            self.diff_scroll = 0;
        }
    }

    pub fn get_selected_diff_file(&self) -> Option<String> {
        let changes: Vec<_> = self.current_messages
            .iter()
            .flat_map(|m| m.file_changes.iter())
            .collect();

        changes.get(self.selected_diff_idx).map(|c| c.file_path.clone())
    }

    pub fn toggle_expand(&mut self) {
        if self.current_tab == Tab::Agents {
            if let Some(i) = self.agent_list_state.selected() {
                if let Some(agent) = self.agents.get(i) {
                    let id = agent.id.clone();
                    if self.expanded_agents.contains(&id) {
                        self.expanded_agents.remove(&id);
                    } else {
                        self.expanded_agents.insert(id);
                    }
                }
            }
        }
    }

    // Get selected session
    pub fn selected_session(&self) -> Option<&Session> {
        self.session_list_state.selected().and_then(|i| self.sessions.get(i))
    }

    // Get selected agent
    pub fn selected_agent(&self) -> Option<&Agent> {
        self.agent_list_state.selected().and_then(|i| self.agents.get(i))
    }

    // Summary stats for dashboard
    pub fn total_sessions(&self) -> usize {
        self.sessions.len()
    }

    pub fn active_agents(&self) -> usize {
        self.agents.iter().filter(|a| a.status == "running").count()
    }

    pub fn pending_tasks(&self) -> usize {
        self.tasks.iter().filter(|t| t.status == "pending").count()
    }

    pub fn completed_tasks(&self) -> usize {
        self.tasks.iter().filter(|t| t.status == "completed").count()
    }

    pub fn today_messages(&self) -> u64 {
        self.daily_stats.last().map(|s| s.message_count).unwrap_or(0)
    }

    pub fn today_tool_calls(&self) -> u64 {
        self.daily_stats.last().map(|s| s.tool_call_count).unwrap_or(0)
    }
}
