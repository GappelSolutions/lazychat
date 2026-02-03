pub mod claude;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub project: String,
    pub project_name: String,
    pub description: Option<String>, // First user message - what the chat is about
    pub custom_name: Option<String>, // User-defined override name
    pub started_at: Option<DateTime<Utc>>,
    pub last_activity: Option<DateTime<Utc>>,
    pub message_count: u64,
    pub status: String,
    pub todos: Vec<TodoItem>, // Session-specific todos
    #[serde(skip)]
    pub file_path: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Agent {
    pub id: String,
    pub session_id: String,
    pub parent_id: Option<String>,
    pub agent_type: String,
    pub status: String,
    pub started_at: Option<DateTime<Utc>>,
    pub description: String,
    pub children: Vec<Agent>,
    #[serde(skip)]
    pub todos: Vec<TodoItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TodoItem {
    pub id: String,
    pub content: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub subject: String,
    pub description: String,
    pub status: String,
    pub agent_id: Option<String>,
    pub created_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyStats {
    pub date: String,
    pub message_count: u64,
    pub session_count: u64,
    pub tool_call_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub display: String,
    pub timestamp: u64,
    pub project: String,
    pub session_id: String,
}

#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub role: String,    // "user" or "assistant"
    pub content: String, // The message text
    pub timestamp: Option<DateTime<Utc>>,
    pub tool_calls: Vec<ToolCall>,
}

#[derive(Debug, Clone)]
pub struct ToolCall {
    pub tool_name: String,
    pub status: String,            // "running", "completed", "error"
    pub file_path: Option<String>, // For Edit/Write tools
}

#[derive(Debug, Clone)]
pub struct FileChange {
    pub path: String,
    pub filename: String,
    pub status: FileStatus,
    pub additions: u32,
    pub deletions: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FileStatus {
    Modified,
    Added,
    Deleted,
    Renamed,
    Untracked,
}

impl ChatMessage {
    pub fn display_content(&self, max_width: usize) -> Vec<String> {
        let mut lines = Vec::new();

        // Format content with word wrapping (char-safe)
        for line in self.content.lines() {
            if line.chars().count() <= max_width {
                lines.push(line.to_string());
            } else {
                // Word wrap
                let words: Vec<&str> = line.split_whitespace().collect();
                let mut current_line = String::new();
                let mut current_len = 0usize;
                for word in words {
                    let word_len = word.chars().count();
                    if current_line.is_empty() {
                        current_line = word.to_string();
                        current_len = word_len;
                    } else if current_len + 1 + word_len <= max_width {
                        current_line.push(' ');
                        current_line.push_str(word);
                        current_len += 1 + word_len;
                    } else {
                        lines.push(current_line);
                        current_line = word.to_string();
                        current_len = word_len;
                    }
                }
                if !current_line.is_empty() {
                    lines.push(current_line);
                }
            }
        }

        // Add tool calls if present
        for tool in &self.tool_calls {
            lines.push(format!("  └─ {} [{}]", tool.tool_name, tool.status));
        }

        lines
    }
}
