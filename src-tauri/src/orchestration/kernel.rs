use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use petgraph::graph::{DiGraph, NodeIndex};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::Mutex;

use crate::orchestration::protocol::{JsonProtocol, Protocol, ProtocolMessage};
use crate::orchestration::events::{OrchestrationEventEmitter, EmitOrchestrationEvent};

/// Status of a task in the orchestration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TaskStatus {
    Pending,
    Ready,      // Dependencies satisfied, ready to run
    Running,
    Completed,
    Failed,
    Cancelled,
    Blocked,    // Waiting for dependencies
}

impl std::fmt::Display for TaskStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TaskStatus::Pending => write!(f, "pending"),
            TaskStatus::Ready => write!(f, "ready"),
            TaskStatus::Running => write!(f, "running"),
            TaskStatus::Completed => write!(f, "completed"),
            TaskStatus::Failed => write!(f, "failed"),
            TaskStatus::Cancelled => write!(f, "cancelled"),
            TaskStatus::Blocked => write!(f, "blocked"),
        }
    }
}

/// Configuration for an agent that will execute a task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub agent_type: String,
    pub model: String,
    pub system_prompt: String,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
    pub capabilities: Vec<String>,
    pub sandbox_profile: Option<String>,
}

/// A task in the orchestration graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: i64,
    pub orchestration_id: i64,
    pub name: String,
    pub description: String,
    pub goal: String,
    pub agent_config: AgentConfig,
    pub status: TaskStatus,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub output: Option<String>,
    pub error: Option<String>,
    pub retry_count: u32,
    pub max_retries: u32,
    pub depends_on: Vec<i64>,  // Task IDs this task depends on
}

/// Message types for inter-agent communication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AgentMessage {
    TaskStarted {
        task_id: i64,
        timestamp: DateTime<Utc>,
    },
    TaskProgress {
        task_id: i64,
        progress: f32,
        message: String,
    },
    TaskOutput {
        task_id: i64,
        output: String,
        artifacts: Vec<String>,  // File paths
    },
    TaskCompleted {
        task_id: i64,
        success: bool,
        output: String,
        timestamp: DateTime<Utc>,
    },
    TaskFailed {
        task_id: i64,
        error: String,
        recoverable: bool,
        timestamp: DateTime<Utc>,
    },
    RequestCollaboration {
        from_task_id: i64,
        to_task_id: i64,
        request: String,
    },
    CollaborationResponse {
        from_task_id: i64,
        to_task_id: i64,
        response: String,
    },
}

/// The orchestration kernel that manages task execution
pub struct OrchestrationKernel {
    db: Arc<Mutex<Connection>>,
    task_graph: Arc<Mutex<DiGraph<i64, ()>>>,  // Node weight is task_id
    task_node_map: Arc<Mutex<HashMap<i64, NodeIndex>>>,  // task_id -> NodeIndex
    active_processes: Arc<Mutex<HashMap<i64, Child>>>,  // task_id -> Child process
    message_queue: Arc<Mutex<Vec<AgentMessage>>>,
    claude_binary_path: String,
    event_emitter: Option<Arc<OrchestrationEventEmitter>>,
}

impl OrchestrationKernel {
    /// Set the event emitter for real-time updates
    pub fn set_event_emitter(&mut self, emitter: OrchestrationEventEmitter) {
        self.event_emitter = Some(Arc::new(emitter));
    }
    
    /// Create a new orchestration kernel with database connection
    pub fn new(db_path: PathBuf, claude_binary_path: String) -> Result<Self> {
        let db = Connection::open(&db_path)
            .context("Failed to open database connection")?;
        
        // Initialize database schema
        Self::init_database(&db)?;
        
        Ok(Self {
            db: Arc::new(Mutex::new(db)),
            task_graph: Arc::new(Mutex::new(DiGraph::new())),
            task_node_map: Arc::new(Mutex::new(HashMap::new())),
            active_processes: Arc::new(Mutex::new(HashMap::new())),
            message_queue: Arc::new(Mutex::new(Vec::new())),
            claude_binary_path,
            event_emitter: None,
        })
    }
    
    /// Initialize database schema for orchestration
    fn init_database(db: &Connection) -> Result<()> {
        db.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS orchestrations (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                root_goal TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT 'running',
                created_at TEXT NOT NULL,
                completed_at TEXT,
                total_tasks INTEGER DEFAULT 0,
                completed_tasks INTEGER DEFAULT 0,
                failed_tasks INTEGER DEFAULT 0
            );
            
            CREATE TABLE IF NOT EXISTS orchestration_tasks (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                orchestration_id INTEGER NOT NULL,
                name TEXT NOT NULL,
                description TEXT NOT NULL,
                goal TEXT NOT NULL,
                agent_config TEXT NOT NULL,  -- JSON
                status TEXT NOT NULL DEFAULT 'pending',
                created_at TEXT NOT NULL,
                started_at TEXT,
                completed_at TEXT,
                output TEXT,
                error TEXT,
                retry_count INTEGER DEFAULT 0,
                max_retries INTEGER DEFAULT 3,
                FOREIGN KEY (orchestration_id) REFERENCES orchestrations(id)
            );
            
            CREATE TABLE IF NOT EXISTS task_dependencies (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                task_id INTEGER NOT NULL,
                depends_on_task_id INTEGER NOT NULL,
                FOREIGN KEY (task_id) REFERENCES orchestration_tasks(id),
                FOREIGN KEY (depends_on_task_id) REFERENCES orchestration_tasks(id),
                UNIQUE(task_id, depends_on_task_id)
            );
            
            CREATE TABLE IF NOT EXISTS task_messages (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                task_id INTEGER NOT NULL,
                message_type TEXT NOT NULL,
                message_data TEXT NOT NULL,  -- JSON
                timestamp TEXT NOT NULL,
                FOREIGN KEY (task_id) REFERENCES orchestration_tasks(id)
            );
            
            CREATE INDEX IF NOT EXISTS idx_orchestration_tasks_status 
                ON orchestration_tasks(orchestration_id, status);
            CREATE INDEX IF NOT EXISTS idx_task_dependencies_task 
                ON task_dependencies(task_id);
            CREATE INDEX IF NOT EXISTS idx_task_dependencies_depends_on 
                ON task_dependencies(depends_on_task_id);
            CREATE INDEX IF NOT EXISTS idx_task_messages_task 
                ON task_messages(task_id);
            "
        )?;
        Ok(())
    }
    
    /// Create a new orchestration with a root goal
    pub async fn create_orchestration(&self, root_goal: String) -> Result<i64> {
        let db = self.db.lock().await;
        
        let orchestration_id = db.query_row(
            "INSERT INTO orchestrations (root_goal, created_at) 
             VALUES (?1, ?2) 
             RETURNING id",
            params![root_goal, Utc::now().to_rfc3339()],
            |row| row.get(0),
        )?;
        
        // Create initial decomposition task
        let decomposer_config = AgentConfig {
            agent_type: "orchestrator".to_string(),
            model: "claude-3-5-sonnet-20241022".to_string(),
            system_prompt: format!(
                "You are an orchestration agent responsible for decomposing complex goals into manageable tasks. \
                 Break down the following goal into a series of concrete, actionable tasks that can be executed by specialized agents. \
                 For each task, specify: name, description, goal, required capabilities, and dependencies. \
                 Root goal: {}",
                root_goal
            ),
            max_tokens: Some(4096),
            temperature: Some(0.7),
            capabilities: vec!["orchestration".to_string(), "planning".to_string()],
            sandbox_profile: None,
        };
        
        let task_id: i64 = db.query_row(
            "INSERT INTO orchestration_tasks 
             (orchestration_id, name, description, goal, agent_config, created_at) 
             VALUES (?1, ?2, ?3, ?4, ?5, ?6) 
             RETURNING id",
            params![
                orchestration_id,
                "Goal Decomposition",
                "Break down the root goal into executable tasks",
                &root_goal,
                serde_json::to_string(&decomposer_config)?,
                Utc::now().to_rfc3339()
            ],
            |row| row.get(0),
        )?;
        
        // Add to task graph
        let mut graph = self.task_graph.lock().await;
        let mut node_map = self.task_node_map.lock().await;
        let node_idx = graph.add_node(task_id);
        node_map.insert(task_id, node_idx);
        
        Ok(orchestration_id)
    }
    
    /// Spawn an agent process for a task
    pub async fn spawn_agent(&self, task_id: i64, agent_config: AgentConfig) -> Result<()> {
        // First get the full task details
        let task = self.get_task(task_id).await?;
        
        // Format the task request using the protocol
        let task_request_json = JsonProtocol::format_task_request(&task)?;
        
        // Get project path from orchestration (you might want to add this to the schema)
        // For now, using a placeholder
        let project_path = PathBuf::from(".");
        
        // Build Claude command with the formatted request
        let mut cmd = Command::new(&self.claude_binary_path);
        cmd.arg("-p")
            .arg(&task_request_json)  // Pass the full JSON task request
            .arg("--model")
            .arg(&agent_config.model)
            .arg("--output-format")
            .arg("stream-json")
            .arg("--dangerously-skip-permissions")
            .current_dir(&project_path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        
        // Add environment for orchestration context
        cmd.env("CLAUDIA_ORCHESTRATION_ID", task.orchestration_id.to_string())
            .env("CLAUDIA_TASK_ID", task_id.to_string())
            .env("CLAUDIA_AGENT_TYPE", &agent_config.agent_type)
            .env("CLAUDIA_PROTOCOL_VERSION", "1.0");
        
        // Spawn the process
        let mut child = cmd.spawn()
            .context("Failed to spawn Claude process")?;
        
        // Get stdout for monitoring
        let stdout = child.stdout.take()
            .ok_or_else(|| anyhow::anyhow!("Failed to get stdout"))?;
        
        // Store the process
        let mut processes = self.active_processes.lock().await;
        processes.insert(task_id, child);
        drop(processes);  // Release lock
        
        // Update task status
        self.update_task_status(task_id, TaskStatus::Running).await?;
        
        // Spawn task to monitor output
        let kernel = self.clone();
        tokio::spawn(async move {
            let reader = BufReader::new(stdout);
            let mut lines = reader.lines();
            
            while let Ok(Some(line)) = lines.next_line().await {
                // Parse agent output using the protocol
                match JsonProtocol::parse_agent_output(&line) {
                    Ok(protocol_msg) => {
                        // Convert ProtocolMessage to AgentMessage for compatibility
                        let agent_msg = match protocol_msg {
                            ProtocolMessage::TaskStarted { task_id, timestamp } => {
                                AgentMessage::TaskStarted { task_id, timestamp }
                            }
                            ProtocolMessage::TaskProgress(ref progress) => {
                                AgentMessage::TaskProgress {
                                    task_id: progress.task_id,
                                    progress: progress.progress,
                                    message: progress.message.clone(),
                                }
                            }
                            ProtocolMessage::TaskResult(ref result) => {
                                AgentMessage::TaskCompleted {
                                    task_id: result.task_id,
                                    success: result.status == TaskStatus::Completed,
                                    output: result.summary.clone(),
                                    timestamp: result.timestamp,
                                }
                            }
                            ProtocolMessage::Error { task_id, ref error, recoverable } => {
                                AgentMessage::TaskFailed {
                                    task_id,
                                    error: error.clone(),
                                    recoverable,
                                    timestamp: Utc::now(),
                                }
                            }
                            ProtocolMessage::TaskLog(ref log) => {
                                // For now, convert logs to output messages
                                AgentMessage::TaskOutput {
                                    task_id: log.task_id,
                                    output: format!("[{}] {}", log.level, log.message),
                                    artifacts: Vec::new(),
                                }
                            }
                            _ => continue,  // Skip other message types
                        };
                        
                        // Emit protocol message event if emitter is available
                        if let Some(emitter) = &kernel.event_emitter {
                            let _ = emitter.emit_from_protocol_message(task.orchestration_id, &protocol_msg);
                        }
                        
                        let _ = kernel.handle_agent_message(task_id, agent_msg).await;
                    }
                    Err(e) => {
                        // Log parsing errors but continue processing
                        log::debug!("Failed to parse agent output: {} - Line: {}", e, line);
                    }
                }
            }
        });
        
        Ok(())
    }
    
    /// Handle messages from agents
    pub async fn handle_agent_message(&self, task_id: i64, message: AgentMessage) -> Result<()> {
        // Get orchestration_id for event emission
        let db = self.db.lock().await;
        let orchestration_id: i64 = db.query_row(
            "SELECT orchestration_id FROM orchestration_tasks WHERE id = ?1",
            params![task_id],
            |row| row.get(0),
        )?;
        
        // Store message in queue
        let mut queue = self.message_queue.lock().await;
        queue.push(message.clone());
        drop(queue);
        
        // Store in database
        db.execute(
            "INSERT INTO task_messages (task_id, message_type, message_data, timestamp) 
             VALUES (?1, ?2, ?3, ?4)",
            params![
                task_id,
                match &message {
                    AgentMessage::TaskStarted { .. } => "task_started",
                    AgentMessage::TaskProgress { .. } => "task_progress",
                    AgentMessage::TaskOutput { .. } => "task_output",
                    AgentMessage::TaskCompleted { .. } => "task_completed",
                    AgentMessage::TaskFailed { .. } => "task_failed",
                    AgentMessage::RequestCollaboration { .. } => "request_collaboration",
                    AgentMessage::CollaborationResponse { .. } => "collaboration_response",
                },
                serde_json::to_string(&message)?,
                Utc::now().to_rfc3339()
            ],
        )?;
        
        // Emit event for the message
        if let Some(emitter) = &self.event_emitter {
            let _ = emitter.emit_from_agent_message(orchestration_id, &message);
        }
        
        // Handle specific message types
        match message {
            AgentMessage::TaskCompleted { task_id, output, .. } => {
                drop(db);  // Release lock
                self.update_task_status(task_id, TaskStatus::Completed).await?;
                self.store_task_output(task_id, output).await?;
                
                // Note: We don't call schedule_tasks here to avoid Send bounds issues
                // The scheduler should run periodically or be triggered elsewhere
            }
            AgentMessage::TaskFailed { task_id, error, recoverable, .. } => {
                drop(db);  // Release lock
                if recoverable {
                    // Check retry count
                    let should_retry = self.should_retry_task(task_id).await?;
                    if should_retry {
                        self.retry_task(task_id).await?;
                    } else {
                        self.update_task_status(task_id, TaskStatus::Failed).await?;
                        self.store_task_error(task_id, error).await?;
                    }
                } else {
                    self.update_task_status(task_id, TaskStatus::Failed).await?;
                    self.store_task_error(task_id, error).await?;
                }
            }
            _ => {}
        }
        
        Ok(())
    }
    
    /// Update task status
    pub async fn update_task_status(&self, task_id: i64, status: TaskStatus) -> Result<()> {
        // Get the current status before updating
        let db = self.db.lock().await;
        let old_status_str: String = db.query_row(
            "SELECT status FROM orchestration_tasks WHERE id = ?1",
            params![task_id],
            |row| row.get(0),
        )?;
        
        let old_status = match old_status_str.as_str() {
            "pending" => TaskStatus::Pending,
            "ready" => TaskStatus::Ready,
            "running" => TaskStatus::Running,
            "completed" => TaskStatus::Completed,
            "failed" => TaskStatus::Failed,
            "cancelled" => TaskStatus::Cancelled,
            "blocked" => TaskStatus::Blocked,
            _ => TaskStatus::Pending,
        };
        
        // Get orchestration_id for event emission
        let orchestration_id: i64 = db.query_row(
            "SELECT orchestration_id FROM orchestration_tasks WHERE id = ?1",
            params![task_id],
            |row| row.get(0),
        )?;
        
        let status_str = match status {
            TaskStatus::Pending => "pending",
            TaskStatus::Ready => "ready",
            TaskStatus::Running => "running",
            TaskStatus::Completed => "completed",
            TaskStatus::Failed => "failed",
            TaskStatus::Cancelled => "cancelled",
            TaskStatus::Blocked => "blocked",
        };
        
        let now = Utc::now().to_rfc3339();
        
        match status {
            TaskStatus::Running => {
                db.execute(
                    "UPDATE orchestration_tasks SET status = ?1, started_at = ?2 WHERE id = ?3",
                    params![status_str, now, task_id],
                )?;
            }
            TaskStatus::Completed | TaskStatus::Failed | TaskStatus::Cancelled => {
                db.execute(
                    "UPDATE orchestration_tasks SET status = ?1, completed_at = ?2 WHERE id = ?3",
                    params![status_str, now, task_id],
                )?;
                
                // Update orchestration stats
                let orchestration_id: i64 = db.query_row(
                    "SELECT orchestration_id FROM orchestration_tasks WHERE id = ?1",
                    params![task_id],
                    |row| row.get(0),
                )?;
                
                if status == TaskStatus::Completed {
                    db.execute(
                        "UPDATE orchestrations SET completed_tasks = completed_tasks + 1 WHERE id = ?1",
                        params![orchestration_id],
                    )?;
                } else if status == TaskStatus::Failed {
                    db.execute(
                        "UPDATE orchestrations SET failed_tasks = failed_tasks + 1 WHERE id = ?1",
                        params![orchestration_id],
                    )?;
                }
            }
            _ => {
                db.execute(
                    "UPDATE orchestration_tasks SET status = ?1 WHERE id = ?2",
                    params![status_str, task_id],
                )?;
            }
        }
        
        drop(db);
        
        // Emit status change event
        if let Some(emitter) = &self.event_emitter {
            let _ = emitter.emit_task_status_changed(orchestration_id, task_id, old_status, status);
        }
        
        Ok(())
    }
    
    /// Get tasks that are ready to run
    pub async fn get_ready_tasks(&self) -> Result<Vec<Task>> {
        let db = self.db.lock().await;
        
        // Find tasks where all dependencies are completed
        let mut stmt = db.prepare(
            "SELECT t.id, t.orchestration_id, t.name, t.description, t.goal, 
                    t.agent_config, t.status, t.created_at, t.started_at, 
                    t.completed_at, t.output, t.error, t.retry_count, t.max_retries
             FROM orchestration_tasks t
             WHERE t.status = 'pending'
               AND NOT EXISTS (
                   SELECT 1 FROM task_dependencies td
                   JOIN orchestration_tasks dt ON td.depends_on_task_id = dt.id
                   WHERE td.task_id = t.id
                     AND dt.status != 'completed'
               )"
        )?;
        
        let tasks = stmt.query_map([], |row| {
            let status_str: String = row.get(6)?;
            let status = match status_str.as_str() {
                "pending" => TaskStatus::Pending,
                "ready" => TaskStatus::Ready,
                "running" => TaskStatus::Running,
                "completed" => TaskStatus::Completed,
                "failed" => TaskStatus::Failed,
                "cancelled" => TaskStatus::Cancelled,
                "blocked" => TaskStatus::Blocked,
                _ => TaskStatus::Pending,
            };
            
            let agent_config: String = row.get(5)?;
            let agent_config: AgentConfig = serde_json::from_str(&agent_config)
                .map_err(|e| rusqlite::Error::FromSqlConversionFailure(
                    5, 
                    rusqlite::types::Type::Text, 
                    Box::new(e)
                ))?;
            
            Ok(Task {
                id: row.get(0)?,
                orchestration_id: row.get(1)?,
                name: row.get(2)?,
                description: row.get(3)?,
                goal: row.get(4)?,
                agent_config,
                status,
                created_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(7)?)
                    .unwrap()
                    .with_timezone(&Utc),
                started_at: row.get::<_, Option<String>>(8)?
                    .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
                    .map(|dt| dt.with_timezone(&Utc)),
                completed_at: row.get::<_, Option<String>>(9)?
                    .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
                    .map(|dt| dt.with_timezone(&Utc)),
                output: row.get(10)?,
                error: row.get(11)?,
                retry_count: row.get(12)?,
                max_retries: row.get(13)?,
                depends_on: Vec::new(), // Will populate separately
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
        
        // Populate dependencies for each task
        let mut result = Vec::new();
        for mut task in tasks {
            let deps: Vec<i64> = db.prepare(
                "SELECT depends_on_task_id FROM task_dependencies WHERE task_id = ?1"
            )?
            .query_map(params![task.id], |row| row.get(0))?
            .collect::<Result<Vec<_>, _>>()?;
            
            task.depends_on = deps;
            result.push(task);
        }
        
        Ok(result)
    }
    
    /// Schedule ready tasks for execution
    pub async fn schedule_tasks(&self) -> Result<()> {
        let ready_tasks = self.get_ready_tasks().await?;
        
        for task in ready_tasks {
            // Check if task is already running
            let processes = self.active_processes.lock().await;
            if processes.contains_key(&task.id) {
                continue;
            }
            drop(processes);
            
            // Update status to ready
            self.update_task_status(task.id, TaskStatus::Ready).await?;
            
            // Spawn agent for the task
            self.spawn_agent(task.id, task.agent_config).await?;
        }
        
        Ok(())
    }
    
    /// Store task output
    async fn store_task_output(&self, task_id: i64, output: String) -> Result<()> {
        let db = self.db.lock().await;
        db.execute(
            "UPDATE orchestration_tasks SET output = ?1 WHERE id = ?2",
            params![output, task_id],
        )?;
        Ok(())
    }
    
    /// Store task error
    async fn store_task_error(&self, task_id: i64, error: String) -> Result<()> {
        let db = self.db.lock().await;
        db.execute(
            "UPDATE orchestration_tasks SET error = ?1 WHERE id = ?2",
            params![error, task_id],
        )?;
        Ok(())
    }
    
    /// Check if a task should be retried
    async fn should_retry_task(&self, task_id: i64) -> Result<bool> {
        let db = self.db.lock().await;
        let (retry_count, max_retries): (u32, u32) = db.query_row(
            "SELECT retry_count, max_retries FROM orchestration_tasks WHERE id = ?1",
            params![task_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
        Ok(retry_count < max_retries)
    }
    
    /// Retry a failed task
    async fn retry_task(&self, task_id: i64) -> Result<()> {
        let db = self.db.lock().await;
        
        // Increment retry count and reset status to pending
        db.execute(
            "UPDATE orchestration_tasks SET retry_count = retry_count + 1, status = 'pending' WHERE id = ?1",
            params![task_id],
        )?;
        
        drop(db);
        
        // Note: We don't re-spawn the agent here to avoid Send bounds issues
        // The task will be picked up by the scheduler when it runs next
        
        Ok(())
    }
    
    /// Add a new task to an orchestration
    pub async fn add_task(
        &self,
        orchestration_id: i64,
        name: String,
        description: String,
        goal: String,
        agent_config: AgentConfig,
        depends_on: Vec<i64>,
    ) -> Result<i64> {
        let db = self.db.lock().await;
        
        // Insert task
        let task_id: i64 = db.query_row(
            "INSERT INTO orchestration_tasks 
             (orchestration_id, name, description, goal, agent_config, created_at) 
             VALUES (?1, ?2, ?3, ?4, ?5, ?6) 
             RETURNING id",
            params![
                orchestration_id,
                name,
                description,
                goal,
                serde_json::to_string(&agent_config)?,
                Utc::now().to_rfc3339()
            ],
            |row| row.get(0),
        )?;
        
        // Add dependencies
        for dep_id in &depends_on {
            db.execute(
                "INSERT INTO task_dependencies (task_id, depends_on_task_id) VALUES (?1, ?2)",
                params![task_id, dep_id],
            )?;
        }
        
        // Update total tasks count
        db.execute(
            "UPDATE orchestrations SET total_tasks = total_tasks + 1 WHERE id = ?1",
            params![orchestration_id],
        )?;
        
        // Add to graph
        let mut graph = self.task_graph.lock().await;
        let mut node_map = self.task_node_map.lock().await;
        
        let node_idx = graph.add_node(task_id);
        node_map.insert(task_id, node_idx);
        
        // Add edges for dependencies
        for dep_id in depends_on {
            if let Some(&dep_idx) = node_map.get(&dep_id) {
                graph.add_edge(dep_idx, node_idx, ());
            }
        }
        
        drop(graph);
        drop(node_map);
        drop(db);
        
        // Get the full task and emit event
        if let Some(emitter) = &self.event_emitter {
            if let Ok(task) = self.get_task(task_id).await {
                let _ = emitter.emit_task_added(orchestration_id, task);
            }
        }
        
        Ok(task_id)
    }
    
    /// Get a task by ID
    pub async fn get_task(&self, task_id: i64) -> Result<Task> {
        let db = self.db.lock().await;
        
        let (id, orchestration_id, name, description, goal, agent_config, status_str, 
             created_at, started_at, completed_at, output, error, retry_count, max_retries): 
            (i64, i64, String, String, String, String, String, String, Option<String>, 
             Option<String>, Option<String>, Option<String>, u32, u32) = 
            db.query_row(
                "SELECT id, orchestration_id, name, description, goal, agent_config, 
                        status, created_at, started_at, completed_at, output, error, 
                        retry_count, max_retries
                 FROM orchestration_tasks WHERE id = ?1",
                params![task_id],
                |row| Ok((
                    row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, 
                    row.get(4)?, row.get(5)?, row.get(6)?, row.get(7)?,
                    row.get(8)?, row.get(9)?, row.get(10)?, row.get(11)?,
                    row.get(12)?, row.get(13)?
                )),
            )?;
        
        let status = match status_str.as_str() {
            "pending" => TaskStatus::Pending,
            "ready" => TaskStatus::Ready,
            "running" => TaskStatus::Running,
            "completed" => TaskStatus::Completed,
            "failed" => TaskStatus::Failed,
            "cancelled" => TaskStatus::Cancelled,
            "blocked" => TaskStatus::Blocked,
            _ => TaskStatus::Pending,
        };
        
        let agent_config: AgentConfig = serde_json::from_str(&agent_config)?;
        
        // Get dependencies
        let depends_on: Vec<i64> = db.prepare(
            "SELECT depends_on_task_id FROM task_dependencies WHERE task_id = ?1"
        )?
        .query_map(params![task_id], |row| row.get(0))?
        .collect::<Result<Vec<_>, _>>()?;
        
        Ok(Task {
            id,
            orchestration_id,
            name,
            description,
            goal,
            agent_config,
            status,
            created_at: DateTime::parse_from_rfc3339(&created_at)
                .unwrap()
                .with_timezone(&Utc),
            started_at: started_at
                .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
                .map(|dt| dt.with_timezone(&Utc)),
            completed_at: completed_at
                .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
                .map(|dt| dt.with_timezone(&Utc)),
            output,
            error,
            retry_count,
            max_retries,
            depends_on,
        })
    }

    /// Get all tasks for an orchestration
    pub async fn get_orchestration_tasks(&self, orchestration_id: i64) -> Result<Vec<Task>> {
        let db = self.db.lock().await;
        
        // Get all task data in a single query instead of collecting IDs and making async calls
        let mut stmt = db.prepare(
            "SELECT id, orchestration_id, name, description, goal, agent_config, 
                    status, created_at, started_at, completed_at, output, error, 
                    retry_count, max_retries
             FROM orchestration_tasks 
             WHERE orchestration_id = ?1 
             ORDER BY id"
        )?;
        
        let task_data: Vec<(i64, i64, String, String, String, String, String, String, 
                           Option<String>, Option<String>, Option<String>, Option<String>, 
                           u32, u32)> = stmt
            .query_map(params![orchestration_id], |row| {
                Ok((
                    row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, 
                    row.get(4)?, row.get(5)?, row.get(6)?, row.get(7)?,
                    row.get(8)?, row.get(9)?, row.get(10)?, row.get(11)?,
                    row.get(12)?, row.get(13)?
                ))
            })?
            .collect::<Result<Vec<_>, _>>()?;
        
        // Get all dependencies in a single query
        let mut deps_stmt = db.prepare(
            "SELECT task_id, depends_on_task_id 
             FROM task_dependencies td
             JOIN orchestration_tasks t ON td.task_id = t.id
             WHERE t.orchestration_id = ?1"
        )?;
        
        let deps_data: Vec<(i64, i64)> = deps_stmt
            .query_map(params![orchestration_id], |row| {
                Ok((row.get(0)?, row.get(1)?))
            })?
            .collect::<Result<Vec<_>, _>>()?;
        
        drop(deps_stmt);
        drop(stmt);
        drop(db);
        
        // Build dependency map
        let mut deps_map: HashMap<i64, Vec<i64>> = HashMap::new();
        for (task_id, dep_id) in deps_data {
            deps_map.entry(task_id).or_insert_with(Vec::new).push(dep_id);
        }
        
        // Build tasks from collected data
        let mut tasks = Vec::new();
        for (id, orchestration_id, name, description, goal, agent_config_str, status_str, 
             created_at, started_at, completed_at, output, error, retry_count, max_retries) in task_data {
            
            let status = match status_str.as_str() {
                "pending" => TaskStatus::Pending,
                "ready" => TaskStatus::Ready,
                "running" => TaskStatus::Running,
                "completed" => TaskStatus::Completed,
                "failed" => TaskStatus::Failed,
                "cancelled" => TaskStatus::Cancelled,
                "blocked" => TaskStatus::Blocked,
                _ => TaskStatus::Pending,
            };
            
            let agent_config: AgentConfig = serde_json::from_str(&agent_config_str)?;
            let depends_on = deps_map.get(&id).cloned().unwrap_or_default();
            
            tasks.push(Task {
                id,
                orchestration_id,
                name,
                description,
                goal,
                agent_config,
                status,
                created_at: DateTime::parse_from_rfc3339(&created_at)
                    .unwrap()
                    .with_timezone(&Utc),
                started_at: started_at
                    .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
                    .map(|dt| dt.with_timezone(&Utc)),
                completed_at: completed_at
                    .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
                    .map(|dt| dt.with_timezone(&Utc)),
                output,
                error,
                retry_count,
                max_retries,
                depends_on,
            });
        }
        
        Ok(tasks)
    }
    
    /// Get orchestration status
    pub async fn get_orchestration_status(&self, orchestration_id: i64) -> Result<serde_json::Value> {
        let db = self.db.lock().await;
        
        let (status, total_tasks, completed_tasks, failed_tasks): (String, i64, i64, i64) = 
            db.query_row(
                "SELECT status, total_tasks, completed_tasks, failed_tasks 
                 FROM orchestrations WHERE id = ?1",
                params![orchestration_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )?;
        
        // Get task breakdown
        let mut stmt = db.prepare(
            "SELECT status, COUNT(*) FROM orchestration_tasks 
             WHERE orchestration_id = ?1 
             GROUP BY status"
        )?;
        
        let task_counts: HashMap<String, i64> = stmt
            .query_map(params![orchestration_id], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
            })?
            .collect::<Result<HashMap<_, _>, _>>()?;
        
        Ok(serde_json::json!({
            "orchestration_id": orchestration_id,
            "status": status,
            "total_tasks": total_tasks,
            "completed_tasks": completed_tasks,
            "failed_tasks": failed_tasks,
            "task_breakdown": task_counts,
            "progress": if total_tasks > 0 {
                (completed_tasks as f64 / total_tasks as f64) * 100.0
            } else {
                0.0
            }
        }))
    }
    
    /// Cancel an orchestration and all its tasks
    pub async fn cancel_orchestration(&self, orchestration_id: i64) -> Result<()> {
        // Kill all active processes for this orchestration
        let db = self.db.lock().await;
        let task_ids: Vec<i64> = db
            .prepare("SELECT id FROM orchestration_tasks WHERE orchestration_id = ?1 AND status = 'running'")?
            .query_map(params![orchestration_id], |row| row.get(0))?
            .collect::<Result<Vec<_>, _>>()?;
        drop(db);
        
        // Kill processes
        let mut processes = self.active_processes.lock().await;
        for task_id in task_ids {
            if let Some(mut child) = processes.remove(&task_id) {
                let _ = child.kill().await;
            }
            self.update_task_status(task_id, TaskStatus::Cancelled).await?;
        }
        drop(processes);
        
        // Update orchestration status
        let db = self.db.lock().await;
        db.execute(
            "UPDATE orchestrations SET status = 'cancelled', completed_at = ?1 WHERE id = ?2",
            params![Utc::now().to_rfc3339(), orchestration_id],
        )?;
        
        // Cancel all pending tasks
        db.execute(
            "UPDATE orchestration_tasks SET status = 'cancelled' 
             WHERE orchestration_id = ?1 AND status IN ('pending', 'ready', 'blocked')",
            params![orchestration_id],
        )?;
        
        Ok(())
    }
}

// Clone implementation for sharing across threads
impl Clone for OrchestrationKernel {
    fn clone(&self) -> Self {
        Self {
            db: Arc::clone(&self.db),
            task_graph: Arc::clone(&self.task_graph),
            task_node_map: Arc::clone(&self.task_node_map),
            active_processes: Arc::clone(&self.active_processes),
            message_queue: Arc::clone(&self.message_queue),
            claude_binary_path: self.claude_binary_path.clone(),
            event_emitter: self.event_emitter.clone(),
        }
    }
}