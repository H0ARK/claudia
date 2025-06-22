use crate::orchestration::agents::{
    WorkerManager, WorkerType, WorkerConfig, WorkerHandle, WorkerStatus, WorkerMetrics
};
use crate::orchestration::{Task, TaskStatus, AgentConfig};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tauri::{AppHandle, State};
use tokio::sync::Mutex;
use chrono::Utc;

/// State wrapper for the worker manager
pub struct WorkerManagerState(pub Arc<Mutex<Option<WorkerManager>>>);

impl Default for WorkerManagerState {
    fn default() -> Self {
        Self(Arc::new(Mutex::new(None)))
    }
}

/// Initialize the worker manager
#[tauri::command]
pub async fn init_worker_manager(
    app: AppHandle,
    state: State<'_, WorkerManagerState>,
) -> Result<String, String> {
    let mut manager_opt = state.0.lock().await;
    
    if manager_opt.is_some() {
        return Ok("Worker manager already initialized".to_string());
    }
    
    // Get Claude binary path
    let claude_path = find_claude_binary(&app)?;
    
    // Create worker manager
    let worker_manager = WorkerManager::new(claude_path);
    
    *manager_opt = Some(worker_manager);
    
    Ok("Worker manager initialized successfully".to_string())
}

/// Spawn a new worker
#[tauri::command]
pub async fn spawn_worker(
    state: State<'_, WorkerManagerState>,
    worker_type: String,
    task_id: i64,
    task_name: String,
    task_description: String,
    task_goal: String,
    config: Option<Value>,
) -> Result<String, String> {
    let manager = {
        let manager_opt = state.0.lock().await;
        manager_opt.as_ref()
            .ok_or_else(|| "Worker manager not initialized".to_string())?
            .clone()
    };
    
    // Parse worker type
    let worker_type = parse_worker_type(&worker_type)?;
    
    // Parse worker config if provided
    let worker_config = if let Some(config_val) = config {
        serde_json::from_value::<WorkerConfig>(config_val)
            .map_err(|e| format!("Invalid worker config: {}", e))?
    } else {
        WorkerConfig {
            worker_type,
            ..Default::default()
        }
    };
    
    // Create a dummy task for the worker
    let task = Task {
        id: task_id,
        orchestration_id: 0, // Would be set from orchestration context
        name: task_name,
        description: task_description,
        goal: task_goal,
        agent_config: AgentConfig {
            agent_type: worker_type.name().to_string(),
            model: worker_config.model.clone().unwrap_or_else(|| worker_type.default_model().to_string()),
            system_prompt: worker_config.custom_system_prompt.clone().unwrap_or_else(|| worker_type.system_prompt()),
            max_tokens: worker_config.max_tokens,
            temperature: worker_config.temperature,
            capabilities: worker_type.capabilities(),
            sandbox_profile: worker_config.sandbox_profile.clone(),
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
    };
    
    // Spawn the worker
    let handle = manager.spawn_worker(&task, worker_type, Some(worker_config)).await
        .map_err(|e| format!("Failed to spawn worker: {}", e))?;
    
    Ok(handle.id)
}

/// Get all active workers
#[tauri::command]
pub async fn get_active_workers(
    state: State<'_, WorkerManagerState>,
) -> Result<Vec<WorkerInfo>, String> {
    let manager = {
        let manager_opt = state.0.lock().await;
        manager_opt.as_ref()
            .ok_or_else(|| "Worker manager not initialized".to_string())?
            .clone()
    };
    
    let handles = manager.get_active_workers().await;
    let mut workers = Vec::new();
    
    for handle in handles {
        let status = handle.status.read().await.clone();
        let metrics = handle.metrics.read().await.clone();
        
        workers.push(WorkerInfo {
            id: handle.id,
            worker_type: worker_type_to_string(handle.worker_type),
            task_id: handle.task_id,
            status: worker_status_to_string(&status),
            started_at: handle.started_at,
            metrics,
        });
    }
    
    Ok(workers)
}

/// Get workers by type
#[tauri::command]
pub async fn get_workers_by_type(
    state: State<'_, WorkerManagerState>,
    worker_type: String,
) -> Result<Vec<WorkerInfo>, String> {
    let manager = {
        let manager_opt = state.0.lock().await;
        manager_opt.as_ref()
            .ok_or_else(|| "Worker manager not initialized".to_string())?
            .clone()
    };
    
    let worker_type = parse_worker_type(&worker_type)?;
    let handles = manager.get_workers_by_type(worker_type).await;
    let mut workers = Vec::new();
    
    for handle in handles {
        let status = handle.status.read().await.clone();
        let metrics = handle.metrics.read().await.clone();
        
        workers.push(WorkerInfo {
            id: handle.id,
            worker_type: worker_type_to_string(handle.worker_type),
            task_id: handle.task_id,
            status: worker_status_to_string(&status),
            started_at: handle.started_at,
            metrics,
        });
    }
    
    Ok(workers)
}

/// Terminate a worker
#[tauri::command]
pub async fn terminate_worker(
    state: State<'_, WorkerManagerState>,
    worker_id: String,
) -> Result<(), String> {
    let manager = {
        let manager_opt = state.0.lock().await;
        manager_opt.as_ref()
            .ok_or_else(|| "Worker manager not initialized".to_string())?
            .clone()
    };
    
    // Find the worker handle
    let handles = manager.get_active_workers().await;
    let handle = handles.into_iter()
        .find(|h| h.id == worker_id)
        .ok_or_else(|| format!("Worker {} not found", worker_id))?;
    
    manager.terminate_worker(&handle).await
        .map_err(|e| format!("Failed to terminate worker: {}", e))
}

/// Send message to worker
#[tauri::command]
pub async fn send_message_to_worker(
    state: State<'_, WorkerManagerState>,
    worker_id: String,
    message: String,
) -> Result<(), String> {
    let manager = {
        let manager_opt = state.0.lock().await;
        manager_opt.as_ref()
            .ok_or_else(|| "Worker manager not initialized".to_string())?
            .clone()
    };
    
    // Find the worker handle
    let handles = manager.get_active_workers().await;
    let handle = handles.into_iter()
        .find(|h| h.id == worker_id)
        .ok_or_else(|| format!("Worker {} not found", worker_id))?;
    
    manager.send_to_worker(&handle, &message).await
        .map_err(|e| format!("Failed to send message to worker: {}", e))
}

/// Get worker metrics
#[tauri::command]
pub async fn get_worker_metrics(
    state: State<'_, WorkerManagerState>,
    worker_id: String,
) -> Result<WorkerMetrics, String> {
    let manager = {
        let manager_opt = state.0.lock().await;
        manager_opt.as_ref()
            .ok_or_else(|| "Worker manager not initialized".to_string())?
            .clone()
    };
    
    // Find the worker handle
    let handles = manager.get_active_workers().await;
    let handle = handles.into_iter()
        .find(|h| h.id == worker_id)
        .ok_or_else(|| format!("Worker {} not found", worker_id))?;
    
    Ok(manager.get_worker_metrics(&handle).await)
}

/// Get global worker metrics
#[tauri::command]
pub async fn get_global_worker_metrics(
    state: State<'_, WorkerManagerState>,
) -> Result<WorkerMetrics, String> {
    let manager = {
        let manager_opt = state.0.lock().await;
        manager_opt.as_ref()
            .ok_or_else(|| "Worker manager not initialized".to_string())?
            .clone()
    };
    
    Ok(manager.get_global_metrics().await)
}

/// Register custom worker type
#[tauri::command]
pub async fn register_custom_worker_type(
    state: State<'_, WorkerManagerState>,
    id: u32,
    config: Value,
) -> Result<(), String> {
    let manager = {
        let manager_opt = state.0.lock().await;
        manager_opt.as_ref()
            .ok_or_else(|| "Worker manager not initialized".to_string())?
            .clone()
    };
    
    let worker_config = serde_json::from_value::<WorkerConfig>(config)
        .map_err(|e| format!("Invalid worker config: {}", e))?;
    
    manager.register_custom_worker_type(id, worker_config).await
        .map_err(|e| format!("Failed to register custom worker type: {}", e))
}

// Helper types and functions

#[derive(serde::Serialize)]
pub struct WorkerInfo {
    pub id: String,
    pub worker_type: String,
    pub task_id: i64,
    pub status: String,
    pub started_at: chrono::DateTime<Utc>,
    pub metrics: WorkerMetrics,
}

fn parse_worker_type(worker_type: &str) -> Result<WorkerType, String> {
    match worker_type.to_lowercase().as_str() {
        "developer" => Ok(WorkerType::Developer),
        "tester" => Ok(WorkerType::Tester),
        "reviewer" => Ok(WorkerType::Reviewer),
        "documentation" => Ok(WorkerType::Documentation),
        custom if custom.starts_with("custom_") => {
            let id_str = custom.strip_prefix("custom_").unwrap();
            let id = id_str.parse::<u32>()
                .map_err(|_| format!("Invalid custom worker type ID: {}", id_str))?;
            Ok(WorkerType::Custom(id))
        }
        _ => Err(format!("Unknown worker type: {}", worker_type)),
    }
}

fn worker_type_to_string(worker_type: WorkerType) -> String {
    match worker_type {
        WorkerType::Developer => "developer".to_string(),
        WorkerType::Tester => "tester".to_string(),
        WorkerType::Reviewer => "reviewer".to_string(),
        WorkerType::Documentation => "documentation".to_string(),
        WorkerType::Custom(id) => format!("custom_{}", id),
    }
}

fn worker_status_to_string(status: &WorkerStatus) -> String {
    match status {
        WorkerStatus::Starting => "starting".to_string(),
        WorkerStatus::Running => "running".to_string(),
        WorkerStatus::Idle => "idle".to_string(),
        WorkerStatus::Busy => "busy".to_string(),
        WorkerStatus::Failed(err) => format!("failed: {}", err),
        WorkerStatus::Stopped => "stopped".to_string(),
    }
}

fn find_claude_binary(app: &AppHandle) -> Result<String, String> {
    // Try to find Claude binary in common locations
    let possible_paths = vec![
        "/usr/local/bin/claude",
        "/opt/homebrew/bin/claude",
        "claude", // Try PATH
    ];
    
    for path in possible_paths {
        if std::process::Command::new(path)
            .arg("--version")
            .output()
            .is_ok()
        {
            return Ok(path.to_string());
        }
    }
    
    // Fallback to a default path
    Ok("claude".to_string())
} 