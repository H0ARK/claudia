use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::{Mutex, RwLock};
use tokio::time::Duration;

use crate::orchestration::{
    AgentConfig, Task, TaskStatus,
    protocol::{JsonProtocol, Protocol, ProtocolMessage},
};

/// Types of worker agents available in the system
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WorkerType {
    Developer,
    Tester,
    Reviewer,
    Documentation,
    Custom(u32), // Custom types with unique IDs
}

impl WorkerType {
    /// Get the display name for this worker type
    pub fn name(&self) -> &str {
        match self {
            WorkerType::Developer => "Developer",
            WorkerType::Tester => "Tester",
            WorkerType::Reviewer => "Reviewer",
            WorkerType::Documentation => "Documentation",
            WorkerType::Custom(_) => "Custom",
        }
    }

    /// Get the default model for this worker type
    pub fn default_model(&self) -> &str {
        match self {
            WorkerType::Developer => "claude-3-5-sonnet-20241022",
            WorkerType::Tester => "claude-3-5-sonnet-20241022",
            WorkerType::Reviewer => "claude-3-5-haiku-20241022",
            WorkerType::Documentation => "claude-3-5-haiku-20241022",
            WorkerType::Custom(_) => "claude-3-5-sonnet-20241022",
        }
    }

    /// Get the system prompt for this worker type
    pub fn system_prompt(&self) -> String {
        match self {
            WorkerType::Developer => DEVELOPER_SYSTEM_PROMPT.to_string(),
            WorkerType::Tester => TESTER_SYSTEM_PROMPT.to_string(),
            WorkerType::Reviewer => REVIEWER_SYSTEM_PROMPT.to_string(),
            WorkerType::Documentation => DOCUMENTATION_SYSTEM_PROMPT.to_string(),
            WorkerType::Custom(_) => CUSTOM_SYSTEM_PROMPT.to_string(),
        }
    }

    /// Get the default capabilities for this worker type
    pub fn capabilities(&self) -> Vec<String> {
        match self {
            WorkerType::Developer => vec![
                "code_generation".to_string(),
                "code_modification".to_string(),
                "debugging".to_string(),
                "refactoring".to_string(),
                "file_operations".to_string(),
                "command_execution".to_string(),
            ],
            WorkerType::Tester => vec![
                "test_generation".to_string(),
                "test_execution".to_string(),
                "coverage_analysis".to_string(),
                "bug_detection".to_string(),
                "file_operations".to_string(),
                "command_execution".to_string(),
            ],
            WorkerType::Reviewer => vec![
                "code_review".to_string(),
                "security_analysis".to_string(),
                "performance_analysis".to_string(),
                "best_practices".to_string(),
                "file_operations".to_string(),
            ],
            WorkerType::Documentation => vec![
                "documentation_generation".to_string(),
                "api_documentation".to_string(),
                "readme_generation".to_string(),
                "diagram_generation".to_string(),
                "file_operations".to_string(),
            ],
            WorkerType::Custom(_) => vec![
                "file_operations".to_string(),
                "command_execution".to_string(),
            ],
        }
    }
}

/// Status of a worker agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WorkerStatus {
    Starting,
    Running,
    Idle,
    Busy,
    Failed(String),
    Stopped,
}

/// Handle to a worker agent
#[derive(Debug, Clone)]
pub struct WorkerHandle {
    pub id: String,
    pub worker_type: WorkerType,
    pub task_id: i64,
    pub status: Arc<RwLock<WorkerStatus>>,
    pub started_at: DateTime<Utc>,
    pub last_heartbeat: Arc<RwLock<DateTime<Utc>>>,
    pub metrics: Arc<RwLock<WorkerMetrics>>,
}

/// Metrics for a worker agent
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WorkerMetrics {
    pub messages_sent: u64,
    pub messages_received: u64,
    pub tasks_completed: u64,
    pub tasks_failed: u64,
    pub total_runtime_seconds: u64,
    pub error_count: u64,
}

/// Configuration for worker agents
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerConfig {
    pub worker_type: WorkerType,
    pub model: Option<String>,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
    pub custom_system_prompt: Option<String>,
    pub additional_capabilities: Vec<String>,
    pub sandbox_profile: Option<String>,
    pub environment_vars: HashMap<String, String>,
    pub working_directory: Option<PathBuf>,
}

impl Default for WorkerConfig {
    fn default() -> Self {
        Self {
            worker_type: WorkerType::Developer,
            model: None,
            max_tokens: Some(4096),
            temperature: Some(0.7),
            custom_system_prompt: None,
            additional_capabilities: Vec::new(),
            sandbox_profile: None,
            environment_vars: HashMap::new(),
            working_directory: None,
        }
    }
}

/// Internal worker process state
struct WorkerProcess {
    handle: WorkerHandle,
    process: Child,
    config: WorkerConfig,
}

/// Manager for worker agents
pub struct WorkerManager {
    /// Path to Claude binary
    claude_binary_path: String,
    /// Active worker processes
    workers: Arc<Mutex<HashMap<String, WorkerProcess>>>,
    /// Worker configuration templates
    config_templates: Arc<RwLock<HashMap<WorkerType, WorkerConfig>>>,
    /// Message handlers for routing
    message_handlers: Arc<Mutex<HashMap<String, Box<dyn Fn(ProtocolMessage) + Send + Sync>>>>,
    /// Global worker metrics
    global_metrics: Arc<RwLock<WorkerMetrics>>,
}

impl WorkerManager {
    /// Create a new worker manager
    pub fn new(claude_binary_path: String) -> Self {
        let mut config_templates = HashMap::new();
        
        // Initialize default configurations for each worker type
        for worker_type in [
            WorkerType::Developer,
            WorkerType::Tester,
            WorkerType::Reviewer,
            WorkerType::Documentation,
        ] {
            config_templates.insert(worker_type, WorkerConfig {
                worker_type,
                model: Some(worker_type.default_model().to_string()),
                ..Default::default()
            });
        }

        Self {
            claude_binary_path,
            workers: Arc::new(Mutex::new(HashMap::new())),
            config_templates: Arc::new(RwLock::new(config_templates)),
            message_handlers: Arc::new(Mutex::new(HashMap::new())),
            global_metrics: Arc::new(RwLock::new(WorkerMetrics::default())),
        }
    }

    /// Spawn a new worker agent
    pub async fn spawn_worker(
        &self,
        task: &Task,
        worker_type: WorkerType,
        config: Option<WorkerConfig>,
    ) -> Result<WorkerHandle> {
        // Use provided config or get default for worker type
        let config = if let Some(cfg) = config {
            cfg
        } else {
            let templates = self.config_templates.read().await;
            templates.get(&worker_type)
                .cloned()
                .unwrap_or_else(|| WorkerConfig {
                    worker_type,
                    ..Default::default()
                })
        };

        // Generate unique worker ID
        let worker_id = format!("{}-{}-{}", worker_type.name(), task.id, Utc::now().timestamp());

        // Create worker handle
        let handle = WorkerHandle {
            id: worker_id.clone(),
            worker_type,
            task_id: task.id,
            status: Arc::new(RwLock::new(WorkerStatus::Starting)),
            started_at: Utc::now(),
            last_heartbeat: Arc::new(RwLock::new(Utc::now())),
            metrics: Arc::new(RwLock::new(WorkerMetrics::default())),
        };

        // Build agent configuration
        let agent_config = self.build_agent_config(&config, task)?;

        // Format task request
        let task_request = JsonProtocol::format_task_request(task)?;

        // Build command
        let mut cmd = Command::new(&self.claude_binary_path);
        cmd.arg("-p")
            .arg(&task_request)
            .arg("--model")
            .arg(agent_config.model)
            .arg("--output-format")
            .arg("stream-json")
            .arg("--dangerously-skip-permissions")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        // Set working directory
        if let Some(ref dir) = config.working_directory {
            cmd.current_dir(dir);
        }

        // Set environment variables
        cmd.env("CLAUDIA_WORKER_ID", &worker_id)
            .env("CLAUDIA_WORKER_TYPE", worker_type.name())
            .env("CLAUDIA_TASK_ID", task.id.to_string())
            .env("CLAUDIA_ORCHESTRATION_ID", task.orchestration_id.to_string());

        for (key, value) in &config.environment_vars {
            cmd.env(key, value);
        }

        // Add sandbox profile if specified
        if let Some(ref profile) = config.sandbox_profile {
            cmd.arg("--sandbox-profile").arg(profile);
        }

        // Spawn the process
        let mut child = cmd.spawn()
            .context("Failed to spawn worker process")?;

        // Get stdout for monitoring
        let stdout = child.stdout.take()
            .ok_or_else(|| anyhow::anyhow!("Failed to get stdout"))?;

        // Update status
        *handle.status.write().await = WorkerStatus::Running;

        // Store worker process
        let mut workers = self.workers.lock().await;
        workers.insert(worker_id.clone(), WorkerProcess {
            handle: handle.clone(),
            process: child,
            config,
        });

        // Start monitoring task
        let manager = self.clone();
        let worker_id_clone = worker_id.clone();
        let handle_clone = handle.clone();
        
        tokio::spawn(async move {
            manager.monitor_worker_output(worker_id_clone, handle_clone, stdout).await;
        });

        // Start heartbeat monitoring
        let manager = self.clone();
        let worker_id_clone = worker_id.clone();
        
        tokio::spawn(async move {
            manager.monitor_worker_heartbeat(worker_id_clone).await;
        });

        Ok(handle)
    }

    /// Monitor worker status
    pub async fn monitor_worker(&self, handle: &WorkerHandle) -> Result<WorkerStatus> {
        Ok(handle.status.read().await.clone())
    }

    /// Send a message to a worker
    pub async fn send_to_worker(&self, handle: &WorkerHandle, message: &str) -> Result<()> {
        // Workers receive messages through their stdin or via file-based IPC
        // For now, we'll log the attempt since workers are primarily task-driven
        log::debug!("Attempting to send message to worker {}: {}", handle.id, message);
        
        // Update metrics
        let mut metrics = handle.metrics.write().await;
        metrics.messages_sent += 1;

        // In a full implementation, this would write to the worker's stdin
        // or use a message queue system
        Ok(())
    }

    /// Terminate a worker
    pub async fn terminate_worker(&self, handle: &WorkerHandle) -> Result<()> {
        let mut workers = self.workers.lock().await;
        
        if let Some(mut worker) = workers.remove(&handle.id) {
            // Update status
            *handle.status.write().await = WorkerStatus::Stopped;
            
            // Kill the process
            worker.process.kill().await
                .context("Failed to kill worker process")?;
            
            log::info!("Terminated worker {}", handle.id);
        }
        
        Ok(())
    }

    /// Restart a worker
    pub async fn restart_worker(&self, handle: &WorkerHandle) -> Result<WorkerHandle> {
        // Get the task and config
        let (task_id, worker_type, config) = {
            let workers = self.workers.lock().await;
            if let Some(worker) = workers.get(&handle.id) {
                (worker.handle.task_id, worker.handle.worker_type, worker.config.clone())
            } else {
                return Err(anyhow::anyhow!("Worker not found"));
            }
        };

        // Terminate the existing worker
        self.terminate_worker(handle).await?;

        // Create a dummy task for respawning (in production, fetch from DB)
        let task = Task {
            id: task_id,
            orchestration_id: 0, // Would be fetched from DB
            name: format!("Restarted task for worker {}", handle.id),
            description: String::new(),
            goal: String::new(),
            agent_config: self.build_agent_config(&config, &Task::default())?,
            status: TaskStatus::Running,
            created_at: Utc::now(),
            started_at: Some(Utc::now()),
            completed_at: None,
            output: None,
            error: None,
            retry_count: 0,
            max_retries: 3,
            depends_on: Vec::new(),
        };

        // Spawn new worker
        self.spawn_worker(&task, worker_type, Some(config)).await
    }

    /// Get all active workers
    pub async fn get_active_workers(&self) -> Vec<WorkerHandle> {
        let workers = self.workers.lock().await;
        workers.values().map(|w| w.handle.clone()).collect()
    }

    /// Get workers by type
    pub async fn get_workers_by_type(&self, worker_type: WorkerType) -> Vec<WorkerHandle> {
        let workers = self.workers.lock().await;
        workers.values()
            .filter(|w| w.handle.worker_type == worker_type)
            .map(|w| w.handle.clone())
            .collect()
    }

    /// Get worker metrics
    pub async fn get_worker_metrics(&self, handle: &WorkerHandle) -> WorkerMetrics {
        handle.metrics.read().await.clone()
    }

    /// Get global metrics
    pub async fn get_global_metrics(&self) -> WorkerMetrics {
        self.global_metrics.read().await.clone()
    }

    /// Register a custom worker type configuration
    pub async fn register_custom_worker_type(&self, id: u32, config: WorkerConfig) -> Result<()> {
        let mut templates = self.config_templates.write().await;
        templates.insert(WorkerType::Custom(id), config);
        Ok(())
    }

    /// Build agent configuration from worker config
    fn build_agent_config(&self, config: &WorkerConfig, _task: &Task) -> Result<AgentConfig> {
        let system_prompt = config.custom_system_prompt
            .clone()
            .unwrap_or_else(|| config.worker_type.system_prompt());

        let mut capabilities = config.worker_type.capabilities();
        capabilities.extend(config.additional_capabilities.clone());

        Ok(AgentConfig {
            agent_type: config.worker_type.name().to_string(),
            model: config.model
                .clone()
                .unwrap_or_else(|| config.worker_type.default_model().to_string()),
            system_prompt,
            max_tokens: config.max_tokens,
            temperature: config.temperature,
            capabilities,
            sandbox_profile: config.sandbox_profile.clone(),
        })
    }

    /// Monitor worker output
    async fn monitor_worker_output(
        &self,
        worker_id: String,
        handle: WorkerHandle,
        stdout: tokio::process::ChildStdout,
    ) {
        let reader = BufReader::new(stdout);
        let mut lines = reader.lines();

        while let Ok(Some(line)) = lines.next_line().await {
            // Update heartbeat
            *handle.last_heartbeat.write().await = Utc::now();

            // Parse protocol message
            match JsonProtocol::parse_agent_output(&line) {
                Ok(msg) => {
                    // Update metrics
                    let mut metrics = handle.metrics.write().await;
                    metrics.messages_received += 1;

                    // Update status based on message
                    match &msg {
                        ProtocolMessage::TaskStarted { .. } => {
                            *handle.status.write().await = WorkerStatus::Busy;
                        }
                        ProtocolMessage::TaskResult(result) => {
                            if result.status == TaskStatus::Completed {
                                metrics.tasks_completed += 1;
                            } else {
                                metrics.tasks_failed += 1;
                            }
                            *handle.status.write().await = WorkerStatus::Idle;
                        }
                        ProtocolMessage::Error { .. } => {
                            metrics.error_count += 1;
                        }
                        _ => {}
                    }

                    // Call registered message handlers
                    let handlers = self.message_handlers.lock().await;
                    if let Some(handler) = handlers.get(&worker_id) {
                        handler(msg);
                    }
                }
                Err(e) => {
                    log::debug!("Failed to parse worker output: {} - Line: {}", e, line);
                }
            }
        }

        // Worker process ended
        *handle.status.write().await = WorkerStatus::Stopped;
        
        // Remove from active workers
        let mut workers = self.workers.lock().await;
        workers.remove(&worker_id);
    }

    /// Monitor worker heartbeat
    async fn monitor_worker_heartbeat(&self, worker_id: String) {
        loop {
            tokio::time::sleep(Duration::from_secs(30)).await;

            let workers = self.workers.lock().await;
            if let Some(worker) = workers.get(&worker_id) {
                let last_heartbeat = *worker.handle.last_heartbeat.read().await;
                let elapsed = Utc::now().signed_duration_since(last_heartbeat);

                if elapsed.num_seconds() > 120 {
                    // Worker hasn't sent a heartbeat in 2 minutes
                    *worker.handle.status.write().await = 
                        WorkerStatus::Failed("Heartbeat timeout".to_string());
                    log::warn!("Worker {} heartbeat timeout", worker_id);
                }
            } else {
                // Worker no longer exists
                break;
            }
        }
    }
}

// Clone implementation for sharing across threads
impl Clone for WorkerManager {
    fn clone(&self) -> Self {
        Self {
            claude_binary_path: self.claude_binary_path.clone(),
            workers: Arc::clone(&self.workers),
            config_templates: Arc::clone(&self.config_templates),
            message_handlers: Arc::clone(&self.message_handlers),
            global_metrics: Arc::clone(&self.global_metrics),
        }
    }
}

// Default implementation for Task (used in restart_worker)
impl Default for Task {
    fn default() -> Self {
        Self {
            id: 0,
            orchestration_id: 0,
            name: String::new(),
            description: String::new(),
            goal: String::new(),
            agent_config: AgentConfig {
                agent_type: "default".to_string(),
                model: "claude-3-5-sonnet-20241022".to_string(),
                system_prompt: String::new(),
                max_tokens: Some(4096),
                temperature: Some(0.7),
                capabilities: Vec::new(),
                sandbox_profile: None,
            },
            status: TaskStatus::Pending,
            created_at: Utc::now(),
            started_at: None,
            completed_at: None,
            output: None,
            error: None,
            retry_count: 0,
            max_retries: 3,
            depends_on: Vec::new(),
        }
    }
}

// System prompts for different agent types
const DEVELOPER_SYSTEM_PROMPT: &str = r#"You are a Developer Agent specialized in implementing code solutions. Your responsibilities include:

1. **Code Implementation**: Write clean, efficient, and well-structured code
2. **Best Practices**: Follow language-specific conventions and design patterns
3. **Error Handling**: Implement robust error handling and validation
4. **Testing Considerations**: Write code that is testable and maintainable
5. **Documentation**: Add clear comments and docstrings to your code

When implementing solutions:
- Analyze requirements thoroughly before coding
- Consider edge cases and potential failures
- Use appropriate data structures and algorithms
- Ensure code is modular and reusable
- Follow SOLID principles where applicable

Your output should include:
- Complete, working code implementations
- Explanations of design decisions
- Any assumptions made
- Suggestions for testing approaches"#;

const TESTER_SYSTEM_PROMPT: &str = r#"You are a Tester Agent specialized in ensuring code quality through comprehensive testing. Your responsibilities include:

1. **Test Design**: Create thorough test plans covering all scenarios
2. **Test Implementation**: Write unit, integration, and end-to-end tests
3. **Edge Cases**: Identify and test boundary conditions and edge cases
4. **Coverage Analysis**: Ensure high code coverage while maintaining test quality
5. **Bug Detection**: Find and report issues with clear reproduction steps

When testing code:
- Write both positive and negative test cases
- Use appropriate testing frameworks and tools
- Mock external dependencies appropriately
- Ensure tests are deterministic and reliable
- Document test purpose and expected outcomes

Your output should include:
- Comprehensive test suites
- Test execution results
- Coverage reports
- Bug reports with severity assessment
- Recommendations for code improvements"#;

const REVIEWER_SYSTEM_PROMPT: &str = r#"You are a Code Reviewer Agent specialized in analyzing code quality, security, and maintainability. Your responsibilities include:

1. **Code Quality**: Assess readability, structure, and adherence to best practices
2. **Security Review**: Identify potential vulnerabilities and security issues
3. **Performance Analysis**: Spot performance bottlenecks and optimization opportunities
4. **Architecture Review**: Evaluate design decisions and architectural patterns
5. **Standards Compliance**: Ensure code follows project and industry standards

When reviewing code:
- Provide constructive feedback with specific examples
- Suggest improvements with code snippets
- Prioritize issues by severity and impact
- Consider both technical and business implications
- Balance perfectionism with pragmatism

Your output should include:
- Detailed review comments organized by category
- Security vulnerability assessment
- Performance improvement suggestions
- Refactoring recommendations
- Overall code quality score with justification"#;

const DOCUMENTATION_SYSTEM_PROMPT: &str = r#"You are a Documentation Agent specialized in creating clear, comprehensive documentation. Your responsibilities include:

1. **API Documentation**: Generate detailed API references with examples
2. **User Guides**: Create easy-to-follow guides for end users
3. **Developer Documentation**: Write technical documentation for developers
4. **Architecture Diagrams**: Create visual representations of system design
5. **README Files**: Produce informative project overviews

When creating documentation:
- Use clear, concise language appropriate for the audience
- Include practical examples and use cases
- Organize content logically with proper headings
- Add diagrams and visualizations where helpful
- Keep documentation up-to-date and version-aware

Your output should include:
- Well-structured documentation files
- Code examples with explanations
- Diagrams in appropriate formats
- Quick-start guides
- Troubleshooting sections"#;

const CUSTOM_SYSTEM_PROMPT: &str = r#"You are a Custom Agent that can be configured for specific tasks. Follow the instructions provided in your task goal and use your capabilities appropriately.

General guidelines:
- Focus on completing the specific task assigned
- Use available tools and capabilities effectively
- Provide clear, actionable output
- Document your process and decisions
- Report any issues or blockers encountered"#;