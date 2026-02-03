use super::{Agent, ChatMessage, DailyStats, FileChange, Session, Task, TodoItem, ToolCall};
use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;
use tokio::fs;

pub struct ClaudeData {
    pub sessions: Vec<Session>,
    pub agents: Vec<Agent>,
    pub tasks: Vec<Task>,
    pub daily_stats: Vec<DailyStats>,
}

#[derive(Debug, Deserialize)]
struct StatsCache {
    #[serde(rename = "dailyActivity")]
    daily_activity: Option<Vec<DailyActivityEntry>>,
}

#[derive(Debug, Deserialize)]
struct DailyActivityEntry {
    date: String,
    #[serde(rename = "messageCount")]
    message_count: u64,
    #[serde(rename = "sessionCount")]
    session_count: u64,
    #[serde(rename = "toolCallCount")]
    tool_call_count: u64,
}

impl ClaudeData {
    pub fn claude_dir() -> PathBuf {
        dirs::home_dir()
            .unwrap_or_default()
            .join(".claude")
    }

    pub async fn load() -> Result<Self> {
        let claude_dir = Self::claude_dir();

        let sessions = Self::load_sessions(&claude_dir).await?;
        let agents = Self::load_agents(&claude_dir).await?;
        let tasks = Self::load_tasks(&claude_dir).await?;
        let daily_stats = Self::load_stats(&claude_dir).await?;

        Ok(Self {
            sessions,
            agents,
            tasks,
            daily_stats,
        })
    }

    /// Load chat messages from a session's transcript file
    pub async fn load_session_messages(session: &Session) -> Result<Vec<ChatMessage>> {
        let file_path = match &session.file_path {
            Some(p) => p.clone(),
            None => return Ok(Vec::new()),
        };

        if !file_path.exists() {
            return Ok(Vec::new());
        }

        let content = fs::read_to_string(&file_path).await?;
        let mut messages = Vec::new();

        for line in content.lines() {
            if line.trim().is_empty() {
                continue;
            }

            if let Ok(json) = serde_json::from_str::<Value>(line) {
                let msg_type = json.get("type").and_then(|v| v.as_str()).unwrap_or("");

                match msg_type {
                    "user" => {
                        if let Some(msg) = json.get("message") {
                            let content = msg.get("content")
                                .and_then(|c| {
                                    if c.is_string() {
                                        c.as_str().map(|s| s.to_string())
                                    } else if c.is_array() {
                                        // Handle array of content blocks
                                        let parts: Vec<String> = c.as_array()
                                            .unwrap_or(&vec![])
                                            .iter()
                                            .filter_map(|block| {
                                                if block.get("type").and_then(|t| t.as_str()) == Some("text") {
                                                    block.get("text").and_then(|t| t.as_str()).map(|s| s.to_string())
                                                } else {
                                                    None
                                                }
                                            })
                                            .collect();
                                        Some(parts.join("\n"))
                                    } else {
                                        None
                                    }
                                })
                                .unwrap_or_default();

                            let timestamp = json.get("timestamp")
                                .and_then(|t| t.as_str())
                                .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
                                .map(|dt| dt.with_timezone(&Utc));

                            if !content.is_empty() {
                                messages.push(ChatMessage {
                                    role: "user".to_string(),
                                    content,
                                    timestamp,
                                    tool_calls: Vec::new(),
                                    file_changes: Vec::new(),
                                    is_thinking: false,
                                });
                            }
                        }
                    }
                    "assistant" => {
                        if let Some(msg) = json.get("message") {
                            let mut content = String::new();
                            let mut tool_calls = Vec::new();
                            let mut file_changes = Vec::new();
                            let mut is_thinking = false;

                            if let Some(content_array) = msg.get("content").and_then(|c| c.as_array()) {
                                for block in content_array {
                                    let block_type = block.get("type").and_then(|t| t.as_str()).unwrap_or("");

                                    match block_type {
                                        "text" => {
                                            if let Some(text) = block.get("text").and_then(|t| t.as_str()) {
                                                if !content.is_empty() {
                                                    content.push('\n');
                                                }
                                                content.push_str(text);
                                            }
                                        }
                                        "thinking" => {
                                            is_thinking = true;
                                            // Optionally include thinking content
                                            if let Some(thinking) = block.get("thinking").and_then(|t| t.as_str()) {
                                                if !content.is_empty() {
                                                    content.push('\n');
                                                }
                                                let truncated: String = thinking.chars().take(100).collect();
                                                content.push_str(&format!("[Thinking: {}...]", truncated));
                                            }
                                        }
                                        "tool_use" => {
                                            let tool_name = block.get("name")
                                                .and_then(|n| n.as_str())
                                                .unwrap_or("unknown")
                                                .to_string();
                                            let tool_id = block.get("id")
                                                .and_then(|i| i.as_str())
                                                .unwrap_or("")
                                                .to_string();

                                            // Extract file changes from Edit/Write tools
                                            if let Some(input) = block.get("input") {
                                                if tool_name == "Edit" {
                                                    let file_path = input.get("file_path")
                                                        .and_then(|p| p.as_str())
                                                        .unwrap_or("")
                                                        .to_string();
                                                    let old_content = input.get("old_string")
                                                        .and_then(|s| s.as_str())
                                                        .unwrap_or("")
                                                        .to_string();
                                                    let new_content = input.get("new_string")
                                                        .and_then(|s| s.as_str())
                                                        .unwrap_or("")
                                                        .to_string();

                                                    if !file_path.is_empty() {
                                                        file_changes.push(FileChange {
                                                            file_path,
                                                            old_content,
                                                            new_content,
                                                            tool_id: tool_id.clone(),
                                                        });
                                                    }
                                                } else if tool_name == "Write" {
                                                    let file_path = input.get("file_path")
                                                        .and_then(|p| p.as_str())
                                                        .unwrap_or("")
                                                        .to_string();
                                                    let new_content = input.get("content")
                                                        .and_then(|s| s.as_str())
                                                        .unwrap_or("")
                                                        .to_string();

                                                    if !file_path.is_empty() {
                                                        file_changes.push(FileChange {
                                                            file_path,
                                                            old_content: String::new(), // Write creates new file
                                                            new_content,
                                                            tool_id: tool_id.clone(),
                                                        });
                                                    }
                                                }
                                            }

                                            tool_calls.push(ToolCall {
                                                tool_name,
                                                status: "completed".to_string(),
                                                result_preview: None,
                                            });
                                        }
                                        _ => {}
                                    }
                                }
                            }

                            let timestamp = json.get("timestamp")
                                .and_then(|t| t.as_str())
                                .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
                                .map(|dt| dt.with_timezone(&Utc));

                            if !content.is_empty() || !tool_calls.is_empty() {
                                messages.push(ChatMessage {
                                    role: "assistant".to_string(),
                                    content: if content.is_empty() && !tool_calls.is_empty() {
                                        format!("[{} tool calls]", tool_calls.len())
                                    } else {
                                        content
                                    },
                                    timestamp,
                                    tool_calls,
                                    file_changes,
                                    is_thinking,
                                });
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        Ok(messages)
    }

    async fn load_sessions(claude_dir: &PathBuf) -> Result<Vec<Session>> {
        let projects_dir = claude_dir.join("projects");
        let mut sessions = Vec::new();

        if !projects_dir.exists() {
            return Ok(sessions);
        }

        let mut dir_entries = fs::read_dir(&projects_dir).await?;

        while let Some(entry) = dir_entries.next_entry().await? {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }

            let project_name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown")
                .replace('-', "/");

            // Read jsonl files in the project directory
            let mut project_entries = fs::read_dir(&path).await?;
            while let Some(file_entry) = project_entries.next_entry().await? {
                let file_path = file_entry.path();
                if file_path.extension().and_then(|e| e.to_str()) != Some("jsonl") {
                    continue;
                }

                let session_id = file_path
                    .file_stem()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown")
                    .to_string();

                // Get file metadata for timestamps
                let metadata = fs::metadata(&file_path).await?;
                let modified = metadata.modified().ok().map(DateTime::<Utc>::from);

                // Count messages (lines in jsonl)
                let content = fs::read_to_string(&file_path).await.unwrap_or_default();
                let message_count = content.lines().count() as u64;

                // Extract first user message as a preview
                let mut status = "idle".to_string();
                for line in content.lines().take(10) {
                    if let Ok(json) = serde_json::from_str::<Value>(line) {
                        if json.get("type").and_then(|v| v.as_str()) == Some("user") {
                            // Check if it's recent (within last 5 minutes)
                            if let Some(ts) = json.get("timestamp").and_then(|t| t.as_str()) {
                                if let Ok(dt) = DateTime::parse_from_rfc3339(ts) {
                                    let age = Utc::now().signed_duration_since(dt.with_timezone(&Utc));
                                    if age.num_minutes() < 5 {
                                        status = "active".to_string();
                                    }
                                }
                            }
                            break;
                        }
                    }
                }

                sessions.push(Session {
                    id: session_id,
                    project: project_name.clone(),
                    project_name: project_name.split('/').last().unwrap_or(&project_name).to_string(),
                    started_at: modified,
                    last_activity: modified,
                    message_count,
                    status,
                    file_path: Some(file_path),
                });
            }
        }

        // Sort by last activity (most recent first)
        sessions.sort_by(|a, b| b.last_activity.cmp(&a.last_activity));

        Ok(sessions)
    }

    async fn load_agents(claude_dir: &PathBuf) -> Result<Vec<Agent>> {
        let todos_dir = claude_dir.join("todos");
        let mut agents = Vec::new();

        if !todos_dir.exists() {
            return Ok(agents);
        }

        let mut dir_entries = fs::read_dir(&todos_dir).await?;
        let mut seen_agents: HashMap<String, bool> = HashMap::new();

        while let Some(entry) = dir_entries.next_entry().await? {
            let path = entry.path();
            let filename = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_string();

            // Parse filename: {sessionId}-agent-{agentId}.json
            if !filename.ends_with(".json") {
                continue;
            }

            let parts: Vec<&str> = filename.trim_end_matches(".json").split("-agent-").collect();
            if parts.len() != 2 {
                continue;
            }

            let session_id = parts[0].to_string();
            let agent_id = parts[1].to_string();

            if seen_agents.contains_key(&agent_id) {
                continue;
            }
            seen_agents.insert(agent_id.clone(), true);

            // Read the file to get todos
            let content = fs::read_to_string(&path).await.unwrap_or_default();
            let todo_values: Vec<Value> = serde_json::from_str(&content).unwrap_or_default();

            let todos: Vec<TodoItem> = todo_values.iter().map(|v| {
                TodoItem {
                    id: v.get("id").and_then(|i| i.as_str()).unwrap_or("").to_string(),
                    content: v.get("subject")
                        .or_else(|| v.get("content"))
                        .and_then(|c| c.as_str())
                        .unwrap_or("")
                        .to_string(),
                    status: v.get("status").and_then(|s| s.as_str()).unwrap_or("pending").to_string(),
                }
            }).collect();

            let has_active = todos.iter().any(|t| t.status == "in_progress");
            let has_todos = !todos.is_empty();

            agents.push(Agent {
                id: agent_id.clone(),
                session_id,
                parent_id: None,
                agent_type: "main".to_string(),
                status: if has_active { "running" } else if has_todos { "active" } else { "idle" }.to_string(),
                started_at: None,
                description: format!("Agent {}", agent_id.chars().take(8).collect::<String>()),
                children: Vec::new(),
                todos,
            });
        }

        // Sort by status (running first, then active, then idle)
        agents.sort_by(|a, b| {
            let order = |s: &str| match s {
                "running" => 0,
                "active" => 1,
                _ => 2,
            };
            order(&a.status).cmp(&order(&b.status))
        });

        Ok(agents)
    }

    async fn load_tasks(claude_dir: &PathBuf) -> Result<Vec<Task>> {
        let tasks_dir = claude_dir.join("tasks");
        let mut tasks = Vec::new();

        if !tasks_dir.exists() {
            return Ok(tasks);
        }

        let mut dir_entries = fs::read_dir(&tasks_dir).await?;

        while let Some(entry) = dir_entries.next_entry().await? {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }

            let task_id = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown")
                .to_string();

            // Try to read task.json or similar
            let task_file = path.join("task.json");
            if task_file.exists() {
                if let Ok(content) = fs::read_to_string(&task_file).await {
                    if let Ok(task_data) = serde_json::from_str::<Value>(&content) {
                        tasks.push(Task {
                            id: task_id.clone(),
                            subject: task_data.get("subject")
                                .and_then(|v| v.as_str())
                                .unwrap_or("Untitled")
                                .to_string(),
                            description: task_data.get("description")
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_string(),
                            status: task_data.get("status")
                                .and_then(|v| v.as_str())
                                .unwrap_or("pending")
                                .to_string(),
                            agent_id: task_data.get("agent_id")
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string()),
                            created_at: None,
                        });
                        continue;
                    }
                }
            }

            // Fallback: create basic task entry
            tasks.push(Task {
                id: task_id.clone(),
                subject: format!("Task {}", task_id.chars().take(8).collect::<String>()),
                description: String::new(),
                status: "unknown".to_string(),
                agent_id: None,
                created_at: None,
            });
        }

        Ok(tasks)
    }

    async fn load_stats(claude_dir: &PathBuf) -> Result<Vec<DailyStats>> {
        let stats_file = claude_dir.join("stats-cache.json");

        if !stats_file.exists() {
            return Ok(Vec::new());
        }

        let content = fs::read_to_string(&stats_file).await?;
        let cache: StatsCache = serde_json::from_str(&content)?;

        Ok(cache.daily_activity.unwrap_or_default().into_iter().map(|entry| {
            DailyStats {
                date: entry.date,
                message_count: entry.message_count,
                session_count: entry.session_count,
                tool_call_count: entry.tool_call_count,
            }
        }).collect())
    }
}
