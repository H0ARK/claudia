use crate::orchestration::{OrchestrationKernel, AgentConfig, TaskStatus, OrchestrationEventEmitter};
use serde_json::Value;
use std::sync::Arc;
use tauri::{AppHandle, Manager, State, Emitter};
use tokio::sync::Mutex;

/// State wrapper for the orchestration kernel
pub struct OrchestrationState(pub Arc<Mutex<Option<OrchestrationKernel>>>);

impl Default for OrchestrationState {
    fn default() -> Self {
        Self(Arc::new(Mutex::new(None)))
    }
}

/// Initialize the orchestration kernel
#[tauri::command]
pub async fn init_orchestration_kernel(
    app: AppHandle,
    state: State<'_, OrchestrationState>,
) -> Result<String, String> {
    let mut kernel_opt = state.0.lock().await;
    
    if kernel_opt.is_some() {
        return Ok("Orchestration kernel already initialized".to_string());
    }
    
    // Get database path
    let app_data_dir = app.path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;
    let db_path = app_data_dir.join("orchestration.db");
    
    // Get Claude binary path from settings or use default
    let claude_path = find_claude_binary(&app)?;
    
    // Create kernel
    let mut kernel = OrchestrationKernel::new(db_path, claude_path)
        .map_err(|e| format!("Failed to create orchestration kernel: {}", e))?;
    
    // Set up event emitter
    let event_emitter = OrchestrationEventEmitter::new(app.clone());
    kernel.set_event_emitter(event_emitter);
    
    *kernel_opt = Some(kernel);
    
    Ok("Orchestration kernel initialized successfully".to_string())
}

/// Create a new orchestration
#[tauri::command]
pub async fn create_orchestration(
    state: State<'_, OrchestrationState>,
    root_goal: String,
) -> Result<i64, String> {
    let kernel = {
        let kernel_opt = state.0.lock().await;
        kernel_opt.as_ref()
            .ok_or_else(|| "Orchestration kernel not initialized".to_string())?
            .clone()
    };
    
    kernel.create_orchestration(root_goal).await
        .map_err(|e| format!("Failed to create orchestration: {}", e))
}

/// Add a task to an orchestration
#[tauri::command]
pub async fn add_orchestration_task(
    state: State<'_, OrchestrationState>,
    orchestration_id: i64,
    name: String,
    description: String,
    goal: String,
    agent_type: String,
    model: String,
    depends_on: Vec<i64>,
) -> Result<i64, String> {
    let kernel = {
        let kernel_opt = state.0.lock().await;
        kernel_opt.as_ref()
            .ok_or_else(|| "Orchestration kernel not initialized".to_string())?
            .clone()
    };
    
    let agent_config = AgentConfig {
        agent_type: agent_type.clone(),
        model,
        system_prompt: format!("You are a {} agent. {}", agent_type, description),
        max_tokens: Some(4096),
        temperature: Some(0.7),
        capabilities: vec![agent_type],
        sandbox_profile: None,
    };
    
    kernel.add_task(orchestration_id, name, description, goal, agent_config, depends_on).await
        .map_err(|e| format!("Failed to add task: {}", e))
}

/// Get orchestration status
#[tauri::command]
pub async fn get_orchestration_status(
    state: State<'_, OrchestrationState>,
    orchestration_id: i64,
) -> Result<Value, String> {
    let kernel = {
        let kernel_opt = state.0.lock().await;
        kernel_opt.as_ref()
            .ok_or_else(|| "Orchestration kernel not initialized".to_string())?
            .clone()
    };
    
    kernel.get_orchestration_status(orchestration_id).await
        .map_err(|e| format!("Failed to get orchestration status: {}", e))
}

/// Schedule tasks for execution
#[tauri::command]
pub async fn schedule_orchestration_tasks(
    state: State<'_, OrchestrationState>,
) -> Result<String, String> {
    let kernel = {
        let kernel_opt = state.0.lock().await;
        kernel_opt.as_ref()
            .ok_or_else(|| "Orchestration kernel not initialized".to_string())?
            .clone()
    };
    
    kernel.schedule_tasks().await
        .map_err(|e| format!("Failed to schedule tasks: {}", e))?;
    
    Ok("Tasks scheduled successfully".to_string())
}

/// Get ready tasks
#[tauri::command]
pub async fn get_ready_tasks(
    state: State<'_, OrchestrationState>,
) -> Result<Vec<crate::orchestration::Task>, String> {
    let kernel = {
        let kernel_opt = state.0.lock().await;
        kernel_opt.as_ref()
            .ok_or_else(|| "Orchestration kernel not initialized".to_string())?
            .clone()
    };
    
    kernel.get_ready_tasks().await
        .map_err(|e| format!("Failed to get ready tasks: {}", e))
}

/// Update task status
#[tauri::command]
pub async fn update_task_status(
    state: State<'_, OrchestrationState>,
    task_id: i64,
    status: String,
) -> Result<(), String> {
    let kernel = {
        let kernel_opt = state.0.lock().await;
        kernel_opt.as_ref()
            .ok_or_else(|| "Orchestration kernel not initialized".to_string())?
            .clone()
    };
    
    let task_status = match status.as_str() {
        "pending" => TaskStatus::Pending,
        "ready" => TaskStatus::Ready,
        "running" => TaskStatus::Running,
        "completed" => TaskStatus::Completed,
        "failed" => TaskStatus::Failed,
        "cancelled" => TaskStatus::Cancelled,
        "blocked" => TaskStatus::Blocked,
        _ => return Err(format!("Invalid task status: {}", status)),
    };
    
    kernel.update_task_status(task_id, task_status).await
        .map_err(|e| format!("Failed to update task status: {}", e))
}

/// Get all tasks for an orchestration
#[tauri::command]
pub async fn get_orchestration_tasks(
    state: State<'_, OrchestrationState>,
    orchestration_id: i64,
) -> Result<Vec<crate::orchestration::Task>, String> {
    let kernel = {
        let kernel_opt = state.0.lock().await;
        kernel_opt.as_ref()
            .ok_or_else(|| "Orchestration kernel not initialized".to_string())?
            .clone()
    };
    
    kernel.get_orchestration_tasks(orchestration_id).await
        .map_err(|e| format!("Failed to get orchestration tasks: {}", e))
}

/// Select a task (for UI interaction)
#[tauri::command]
pub async fn select_task(
    app: AppHandle,
    task_id: i64,
) -> Result<(), String> {
    // Emit an event for UI components to handle
    app.emit("task-selected", task_id)
        .map_err(|e| format!("Failed to emit task-selected event: {}", e))
}

/// Cancel an orchestration
#[tauri::command]
pub async fn cancel_orchestration(
    state: State<'_, OrchestrationState>,
    orchestration_id: i64,
) -> Result<(), String> {
    let kernel = {
        let kernel_opt = state.0.lock().await;
        kernel_opt.as_ref()
            .ok_or_else(|| "Orchestration kernel not initialized".to_string())?
            .clone()
    };
    
    kernel.cancel_orchestration(orchestration_id).await
        .map_err(|e| format!("Failed to cancel orchestration: {}", e))
}

/// Handle agent message (for testing/debugging)
#[tauri::command]
pub async fn send_agent_message(
    state: State<'_, OrchestrationState>,
    task_id: i64,
    message_type: String,
    data: Value,
) -> Result<(), String> {
    let kernel = {
        let kernel_opt = state.0.lock().await;
        kernel_opt.as_ref()
            .ok_or_else(|| "Orchestration kernel not initialized".to_string())?
            .clone()
    };
    
    use crate::orchestration::AgentMessage;
    use chrono::Utc;
    
    let message = match message_type.as_str() {
        "task_completed" => {
            let output = data.get("output")
                .and_then(|v| v.as_str())
                .ok_or("Missing output field")?;
            AgentMessage::TaskCompleted {
                task_id,
                success: true,
                output: output.to_string(),
                timestamp: Utc::now(),
            }
        }
        "task_failed" => {
            let error = data.get("error")
                .and_then(|v| v.as_str())
                .ok_or("Missing error field")?;
            let recoverable = data.get("recoverable")
                .and_then(|v| v.as_bool())
                .unwrap_or(true);
            AgentMessage::TaskFailed {
                task_id,
                error: error.to_string(),
                recoverable,
                timestamp: Utc::now(),
            }
        }
        _ => return Err(format!("Unsupported message type: {}", message_type)),
    };
    
    kernel.handle_agent_message(task_id, message).await
        .map_err(|e| format!("Failed to handle agent message: {}", e))
}

/// Helper function to find Claude binary (reuse from claude.rs)
fn find_claude_binary(app: &AppHandle) -> Result<String, String> {
    // First check if we have a stored path in the database
    if let Ok(app_data_dir) = app.path().app_data_dir() {
        let db_path = app_data_dir.join("agents.db");
        if db_path.exists() {
            if let Ok(conn) = rusqlite::Connection::open(&db_path) {
                if let Ok(stored_path) = conn.query_row(
                    "SELECT value FROM app_settings WHERE key = 'claude_binary_path'",
                    [],
                    |row| row.get::<_, String>(0),
                ) {
                    if std::path::PathBuf::from(&stored_path).exists() {
                        return Ok(stored_path);
                    }
                }
            }
        }
    }
    
    // Common installation paths for claude
    let paths_to_check = vec![
        "/usr/local/bin/claude",
        "/opt/homebrew/bin/claude",
        "/usr/bin/claude",
        "/bin/claude",
    ];
    
    // Also check user-specific paths
    if let Ok(home) = std::env::var("HOME") {
        let user_paths = vec![
            format!("{}/.claude/local/claude", home),
            format!("{}/.local/bin/claude", home),
            format!("{}/.npm-global/bin/claude", home),
            format!("{}/.yarn/bin/claude", home),
            format!("{}/.bun/bin/claude", home),
            format!("{}/bin/claude", home),
        ];
        
        // Check all paths
        let all_paths: Vec<&str> = paths_to_check.iter()
            .copied()
            .chain(user_paths.iter().map(|s| s.as_str()))
            .collect();
            
        for path in all_paths {
            if std::path::PathBuf::from(path).exists() {
                return Ok(path.to_string());
            }
        }
    }
    
    // Default fallback
    Ok("claude".to_string())
}