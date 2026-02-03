use super::{Agent, ChatMessage, Session, TodoItem, ToolCall};
use anyhow::Result;
use chrono::{DateTime, Utc};
use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;
use tokio::fs;

pub struct ClaudeData {
    pub sessions: Vec<Session>,
    pub agents: Vec<Agent>,
}

impl ClaudeData {
    pub fn claude_dir() -> PathBuf {
        dirs::home_dir().unwrap_or_default().join(".claude")
    }

    pub async fn load() -> Result<Self> {
        let claude_dir = Self::claude_dir();

        let mut sessions = Self::load_sessions(&claude_dir).await?;
        let agents = Self::load_agents(&claude_dir).await?;

        // Load history to get first user messages as descriptions
        let history = Self::load_history(&claude_dir).await.unwrap_or_default();

        // Load tasks from ~/.claude/tasks/{sessionId}/*.json
        let tasks_by_session = Self::load_tasks_by_session(&claude_dir)
            .await
            .unwrap_or_default();

        // Populate todos and descriptions into each session
        for session in &mut sessions {
            // Add todos from agents (old system: ~/.claude/todos/)
            let mut session_todos: Vec<TodoItem> = agents
                .iter()
                .filter(|a| a.session_id == session.id)
                .flat_map(|a| a.todos.clone())
                .collect();

            // Add tasks (new system: ~/.claude/tasks/)
            if let Some(tasks) = tasks_by_session.get(&session.id) {
                session_todos.extend(tasks.clone());
            }

            session.todos = session_todos;

            // Add description from history (first user message)
            if let Some(desc) = history.get(&session.id) {
                session.description = Some(desc.clone());
            }
        }

        Ok(Self { sessions, agents })
    }

    /// Load tasks from ~/.claude/tasks/{sessionId}/*.json
    async fn load_tasks_by_session(
        claude_dir: &std::path::Path,
    ) -> Result<HashMap<String, Vec<TodoItem>>> {
        let tasks_dir = claude_dir.join("tasks");
        let mut tasks_map: HashMap<String, Vec<TodoItem>> = HashMap::new();

        if !tasks_dir.exists() {
            return Ok(tasks_map);
        }

        let mut dir_entries = fs::read_dir(&tasks_dir).await?;

        while let Some(entry) = dir_entries.next_entry().await? {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }

            let session_id = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_string();

            let mut tasks = Vec::new();

            // Read all .json files in the session's tasks directory
            let mut task_files = fs::read_dir(&path).await?;
            while let Some(task_entry) = task_files.next_entry().await? {
                let task_path = task_entry.path();
                if task_path.extension().and_then(|e| e.to_str()) != Some("json") {
                    continue;
                }

                if let Ok(content) = fs::read_to_string(&task_path).await {
                    if let Ok(task_data) = serde_json::from_str::<Value>(&content) {
                        let subject = task_data
                            .get("subject")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        let status = task_data
                            .get("status")
                            .and_then(|v| v.as_str())
                            .unwrap_or("pending")
                            .to_string();
                        let id = task_data
                            .get("id")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();

                        if !subject.is_empty() {
                            tasks.push(TodoItem {
                                id,
                                content: subject,
                                status,
                            });
                        }
                    }
                }
            }

            if !tasks.is_empty() {
                tasks_map.insert(session_id, tasks);
            }
        }

        Ok(tasks_map)
    }

    /// Load history.jsonl to extract first user messages per session
    async fn load_history(claude_dir: &std::path::Path) -> Result<HashMap<String, String>> {
        let history_file = claude_dir.join("history.jsonl");
        let mut descriptions: HashMap<String, String> = HashMap::new();

        if !history_file.exists() {
            return Ok(descriptions);
        }

        let content = fs::read_to_string(&history_file).await?;

        for line in content.lines() {
            if let Ok(json) = serde_json::from_str::<Value>(line) {
                let session_id = json
                    .get("sessionId")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();

                // Skip if we already have a description for this session
                if descriptions.contains_key(&session_id) {
                    continue;
                }

                if let Some(display) = json.get("display").and_then(|v| v.as_str()) {
                    // Skip commands (start with /)
                    if display.starts_with('/') || display.starts_with('<') {
                        continue;
                    }
                    // Skip very short messages
                    if display.len() < 5 {
                        continue;
                    }
                    descriptions.insert(session_id, display.to_string());
                }
            }
        }

        Ok(descriptions)
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
                            let content = msg
                                .get("content")
                                .and_then(|c| {
                                    if c.is_string() {
                                        c.as_str().map(|s| s.to_string())
                                    } else if c.is_array() {
                                        // Handle array of content blocks
                                        let parts: Vec<String> = c
                                            .as_array()
                                            .unwrap_or(&vec![])
                                            .iter()
                                            .filter_map(|block| {
                                                if block.get("type").and_then(|t| t.as_str())
                                                    == Some("text")
                                                {
                                                    block
                                                        .get("text")
                                                        .and_then(|t| t.as_str())
                                                        .map(|s| s.to_string())
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

                            let timestamp = json
                                .get("timestamp")
                                .and_then(|t| t.as_str())
                                .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
                                .map(|dt| dt.with_timezone(&Utc));

                            if !content.is_empty() {
                                messages.push(ChatMessage {
                                    role: "user".to_string(),
                                    content,
                                    timestamp,
                                    tool_calls: Vec::new(),
                                });
                            }
                        }
                    }
                    "assistant" => {
                        if let Some(msg) = json.get("message") {
                            let mut content = String::new();
                            let mut tool_calls = Vec::new();

                            if let Some(content_array) =
                                msg.get("content").and_then(|c| c.as_array())
                            {
                                for block in content_array {
                                    let block_type =
                                        block.get("type").and_then(|t| t.as_str()).unwrap_or("");

                                    match block_type {
                                        "text" => {
                                            if let Some(text) =
                                                block.get("text").and_then(|t| t.as_str())
                                            {
                                                if !content.is_empty() {
                                                    content.push('\n');
                                                }
                                                content.push_str(text);
                                            }
                                        }
                                        "thinking" => {
                                            if let Some(thinking) =
                                                block.get("thinking").and_then(|t| t.as_str())
                                            {
                                                if !content.is_empty() {
                                                    content.push('\n');
                                                }
                                                let truncated: String =
                                                    thinking.chars().take(100).collect();
                                                content.push_str(&format!(
                                                    "[Thinking: {truncated}...]"
                                                ));
                                            }
                                        }
                                        "tool_use" => {
                                            let tool_name = block
                                                .get("name")
                                                .and_then(|n| n.as_str())
                                                .unwrap_or("unknown")
                                                .to_string();

                                            // Extract file_path from Edit/Write tool inputs
                                            let file_path =
                                                if tool_name == "Edit" || tool_name == "Write" {
                                                    block
                                                        .get("input")
                                                        .and_then(|i| i.get("file_path"))
                                                        .and_then(|p| p.as_str())
                                                        .map(|s| s.to_string())
                                                } else {
                                                    None
                                                };

                                            tool_calls.push(ToolCall {
                                                tool_name,
                                                status: "completed".to_string(),
                                                file_path,
                                            });
                                        }
                                        _ => {}
                                    }
                                }
                            }

                            let timestamp = json
                                .get("timestamp")
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

                // Get file metadata for timestamps and size
                let metadata = fs::metadata(&file_path).await?;
                let modified = metadata.modified().ok().map(DateTime::<Utc>::from);
                let file_size = metadata.len();

                // Estimate message count from file size (avg ~500 bytes per line)
                let message_count = (file_size / 500).max(1);

                // Check for state file first (written by Claude hooks)
                // Then fall back to file modification time
                let state_file = claude_dir
                    .join("session-state")
                    .join(format!("{}.state", &session_id));
                let status = if state_file.exists() {
                    // Read state from hook-written file
                    std::fs::read_to_string(&state_file)
                        .map(|s| s.trim().to_string())
                        .unwrap_or_else(|_| "idle".to_string())
                } else if let Some(mod_time) = &modified {
                    // Fall back to time-based detection
                    // working = < 10 sec (actively processing)
                    // active = < 2 min (recent activity)
                    // idle = 2-30 min (waiting)
                    // inactive = > 30 min (old)
                    let age = chrono::Utc::now().signed_duration_since(*mod_time);
                    if age.num_seconds() < 10 {
                        "working".to_string()
                    } else if age.num_seconds() < 120 {
                        "active".to_string()
                    } else if age.num_minutes() < 30 {
                        "idle".to_string()
                    } else {
                        "inactive".to_string()
                    }
                } else {
                    "inactive".to_string()
                };

                sessions.push(Session {
                    id: session_id,
                    project: project_name.clone(),
                    project_name: project_name
                        .split('/')
                        .last()
                        .unwrap_or(&project_name)
                        .to_string(),
                    description: None, // Will be populated from history.jsonl
                    custom_name: None,
                    started_at: modified,
                    last_activity: modified,
                    message_count,
                    status,
                    todos: Vec::new(), // Will be populated after loading all sessions
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

            let parts: Vec<&str> = filename
                .trim_end_matches(".json")
                .split("-agent-")
                .collect();
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

            let todos: Vec<TodoItem> = todo_values
                .iter()
                .map(|v| TodoItem {
                    id: v
                        .get("id")
                        .and_then(|i| i.as_str())
                        .unwrap_or("")
                        .to_string(),
                    content: v
                        .get("subject")
                        .or_else(|| v.get("content"))
                        .and_then(|c| c.as_str())
                        .unwrap_or("")
                        .to_string(),
                    status: v
                        .get("status")
                        .and_then(|s| s.as_str())
                        .unwrap_or("pending")
                        .to_string(),
                })
                .collect();

            let has_active = todos.iter().any(|t| t.status == "in_progress");
            let has_todos = !todos.is_empty();

            agents.push(Agent {
                id: agent_id.clone(),
                session_id,
                parent_id: None,
                agent_type: "main".to_string(),
                status: if has_active {
                    "running"
                } else if has_todos {
                    "active"
                } else {
                    "idle"
                }
                .to_string(),
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
}
