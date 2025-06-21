use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, Emitter};
use chrono::{DateTime, Utc};
use anyhow::{Context, Result};

use super::{TaskStatus, Task, AgentMessage, ProtocolMessage, TaskProgress, TaskLog, Artifact};

/// Event types for orchestration updates
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum OrchestrationEvent {
    /// Task status has changed
    TaskStatusChanged {
        orchestration_id: i64,
        task_id: i64,
        old_status: TaskStatus,
        new_status: TaskStatus,
        timestamp: DateTime<Utc>,
    },
    
    /// New message received from an agent
    TaskMessage {
        orchestration_id: i64,
        task_id: i64,
        message_type: String,
        message: String,
        timestamp: DateTime<Utc>,
    },
    
    /// Task progress update
    TaskProgress {
        orchestration_id: i64,
        task_id: i64,
        progress: f32,
        message: String,
        timestamp: DateTime<Utc>,
    },
    
    /// Task log entry
    TaskLog {
        orchestration_id: i64,
        task_id: i64,
        level: String,
        message: String,
        timestamp: DateTime<Utc>,
    },
    
    /// Task has produced artifacts
    TaskArtifacts {
        orchestration_id: i64,
        task_id: i64,
        artifacts: Vec<Artifact>,
        timestamp: DateTime<Utc>,
    },
    
    /// Orchestration status changed
    OrchestrationStatusChanged {
        orchestration_id: i64,
        status: String,
        total_tasks: i64,
        completed_tasks: i64,
        failed_tasks: i64,
        progress: f32,
        timestamp: DateTime<Utc>,
    },
    
    /// New task added to orchestration
    TaskAdded {
        orchestration_id: i64,
        task: Task,
        timestamp: DateTime<Utc>,
    },
    
    /// Task dependency resolved
    TaskDependencyResolved {
        orchestration_id: i64,
        task_id: i64,
        dependency_id: i64,
        timestamp: DateTime<Utc>,
    },
    
    /// Error occurred
    OrchestrationError {
        orchestration_id: i64,
        task_id: Option<i64>,
        error: String,
        recoverable: bool,
        timestamp: DateTime<Utc>,
    },
}

/// Event emitter for orchestration events
pub struct OrchestrationEventEmitter {
    app_handle: AppHandle,
}

impl OrchestrationEventEmitter {
    /// Create a new event emitter
    pub fn new(app_handle: AppHandle) -> Self {
        Self { app_handle }
    }
    
    /// Emit a task status change event
    pub fn emit_task_status_changed(
        &self,
        orchestration_id: i64,
        task_id: i64,
        old_status: TaskStatus,
        new_status: TaskStatus,
    ) -> Result<()> {
        let event = OrchestrationEvent::TaskStatusChanged {
            orchestration_id,
            task_id,
            old_status,
            new_status,
            timestamp: Utc::now(),
        };
        
        self.emit_event(event)
    }
    
    /// Emit a task message event
    pub fn emit_task_message(
        &self,
        orchestration_id: i64,
        task_id: i64,
        message_type: String,
        message: String,
    ) -> Result<()> {
        let event = OrchestrationEvent::TaskMessage {
            orchestration_id,
            task_id,
            message_type,
            message,
            timestamp: Utc::now(),
        };
        
        self.emit_event(event)
    }
    
    /// Emit a task progress event
    pub fn emit_task_progress(
        &self,
        orchestration_id: i64,
        task_id: i64,
        progress: f32,
        message: String,
    ) -> Result<()> {
        let event = OrchestrationEvent::TaskProgress {
            orchestration_id,
            task_id,
            progress,
            message,
            timestamp: Utc::now(),
        };
        
        self.emit_event(event)
    }
    
    /// Emit a task log event
    pub fn emit_task_log(
        &self,
        orchestration_id: i64,
        task_id: i64,
        level: String,
        message: String,
    ) -> Result<()> {
        let event = OrchestrationEvent::TaskLog {
            orchestration_id,
            task_id,
            level,
            message,
            timestamp: Utc::now(),
        };
        
        self.emit_event(event)
    }
    
    /// Emit a task artifacts event
    pub fn emit_task_artifacts(
        &self,
        orchestration_id: i64,
        task_id: i64,
        artifacts: Vec<Artifact>,
    ) -> Result<()> {
        let event = OrchestrationEvent::TaskArtifacts {
            orchestration_id,
            task_id,
            artifacts,
            timestamp: Utc::now(),
        };
        
        self.emit_event(event)
    }
    
    /// Emit an orchestration status change event
    pub fn emit_orchestration_status_changed(
        &self,
        orchestration_id: i64,
        status: String,
        total_tasks: i64,
        completed_tasks: i64,
        failed_tasks: i64,
    ) -> Result<()> {
        let progress = if total_tasks > 0 {
            (completed_tasks as f32 / total_tasks as f32) * 100.0
        } else {
            0.0
        };
        
        let event = OrchestrationEvent::OrchestrationStatusChanged {
            orchestration_id,
            status,
            total_tasks,
            completed_tasks,
            failed_tasks,
            progress,
            timestamp: Utc::now(),
        };
        
        self.emit_event(event)
    }
    
    /// Emit a task added event
    pub fn emit_task_added(
        &self,
        orchestration_id: i64,
        task: Task,
    ) -> Result<()> {
        let event = OrchestrationEvent::TaskAdded {
            orchestration_id,
            task,
            timestamp: Utc::now(),
        };
        
        self.emit_event(event)
    }
    
    /// Emit a task dependency resolved event
    pub fn emit_task_dependency_resolved(
        &self,
        orchestration_id: i64,
        task_id: i64,
        dependency_id: i64,
    ) -> Result<()> {
        let event = OrchestrationEvent::TaskDependencyResolved {
            orchestration_id,
            task_id,
            dependency_id,
            timestamp: Utc::now(),
        };
        
        self.emit_event(event)
    }
    
    /// Emit an orchestration error event
    pub fn emit_orchestration_error(
        &self,
        orchestration_id: i64,
        task_id: Option<i64>,
        error: String,
        recoverable: bool,
    ) -> Result<()> {
        let event = OrchestrationEvent::OrchestrationError {
            orchestration_id,
            task_id,
            error,
            recoverable,
            timestamp: Utc::now(),
        };
        
        self.emit_event(event)
    }
    
    /// Emit an event to all listeners
    fn emit_event(&self, event: OrchestrationEvent) -> Result<()> {
        // Emit to specific orchestration channel
        let orchestration_id = match &event {
            OrchestrationEvent::TaskStatusChanged { orchestration_id, .. } => *orchestration_id,
            OrchestrationEvent::TaskMessage { orchestration_id, .. } => *orchestration_id,
            OrchestrationEvent::TaskProgress { orchestration_id, .. } => *orchestration_id,
            OrchestrationEvent::TaskLog { orchestration_id, .. } => *orchestration_id,
            OrchestrationEvent::TaskArtifacts { orchestration_id, .. } => *orchestration_id,
            OrchestrationEvent::OrchestrationStatusChanged { orchestration_id, .. } => *orchestration_id,
            OrchestrationEvent::TaskAdded { orchestration_id, .. } => *orchestration_id,
            OrchestrationEvent::TaskDependencyResolved { orchestration_id, .. } => *orchestration_id,
            OrchestrationEvent::OrchestrationError { orchestration_id, .. } => *orchestration_id,
        };
        
        let channel = format!("orchestration:{}", orchestration_id);
        self.app_handle.emit(&channel, &event)
            .context("Failed to emit orchestration event")?;
        
        // Also emit to global orchestration channel
        self.app_handle.emit("orchestration:all", &event)
            .context("Failed to emit to global orchestration channel")?;
        
        Ok(())
    }
}

/// Helper functions to emit events from different contexts
impl OrchestrationEventEmitter {
    /// Create event from AgentMessage
    pub fn emit_from_agent_message(
        &self,
        orchestration_id: i64,
        message: &AgentMessage,
    ) -> Result<()> {
        match message {
            AgentMessage::TaskStarted { task_id, .. } => {
                self.emit_task_status_changed(
                    orchestration_id,
                    *task_id,
                    TaskStatus::Ready,
                    TaskStatus::Running,
                )?;
            }
            AgentMessage::TaskProgress { task_id, progress, message } => {
                self.emit_task_progress(
                    orchestration_id,
                    *task_id,
                    *progress,
                    message.clone(),
                )?;
            }
            AgentMessage::TaskOutput { task_id, output, artifacts } => {
                self.emit_task_message(
                    orchestration_id,
                    *task_id,
                    "output".to_string(),
                    output.clone(),
                )?;
                
                // Convert file paths to artifacts if needed
                if !artifacts.is_empty() {
                    // This would need proper artifact conversion
                    self.emit_task_message(
                        orchestration_id,
                        *task_id,
                        "artifacts".to_string(),
                        format!("Produced {} artifacts", artifacts.len()),
                    )?;
                }
            }
            AgentMessage::TaskCompleted { task_id, success, output, .. } => {
                self.emit_task_status_changed(
                    orchestration_id,
                    *task_id,
                    TaskStatus::Running,
                    if *success { TaskStatus::Completed } else { TaskStatus::Failed },
                )?;
                
                self.emit_task_message(
                    orchestration_id,
                    *task_id,
                    "completion".to_string(),
                    output.clone(),
                )?;
            }
            AgentMessage::TaskFailed { task_id, error, recoverable, .. } => {
                self.emit_orchestration_error(
                    orchestration_id,
                    Some(*task_id),
                    error.clone(),
                    *recoverable,
                )?;
            }
            _ => {
                // Handle other message types as needed
            }
        }
        
        Ok(())
    }
    
    /// Create event from ProtocolMessage
    pub fn emit_from_protocol_message(
        &self,
        orchestration_id: i64,
        message: &ProtocolMessage,
    ) -> Result<()> {
        match message {
            ProtocolMessage::TaskStarted { task_id, .. } => {
                self.emit_task_status_changed(
                    orchestration_id,
                    *task_id,
                    TaskStatus::Ready,
                    TaskStatus::Running,
                )?;
            }
            ProtocolMessage::TaskProgress(progress) => {
                self.emit_task_progress(
                    orchestration_id,
                    progress.task_id,
                    progress.progress,
                    progress.message.clone(),
                )?;
            }
            ProtocolMessage::TaskLog(log) => {
                self.emit_task_log(
                    orchestration_id,
                    log.task_id,
                    log.level.to_string(),
                    log.message.clone(),
                )?;
            }
            ProtocolMessage::TaskResult(result) => {
                let new_status = match result.status {
                    TaskStatus::Completed => TaskStatus::Completed,
                    TaskStatus::Failed => TaskStatus::Failed,
                    _ => TaskStatus::Running,
                };
                
                self.emit_task_status_changed(
                    orchestration_id,
                    result.task_id,
                    TaskStatus::Running,
                    new_status,
                )?;
                
                if !result.artifacts.is_empty() {
                    self.emit_task_artifacts(
                        orchestration_id,
                        result.task_id,
                        result.artifacts.clone(),
                    )?;
                }
            }
            ProtocolMessage::Error { task_id, error, recoverable } => {
                self.emit_orchestration_error(
                    orchestration_id,
                    Some(*task_id),
                    error.clone(),
                    *recoverable,
                )?;
            }
            _ => {
                // Handle other message types as needed
            }
        }
        
        Ok(())
    }
}

/// Extension trait for convenient event emission
pub trait EmitOrchestrationEvent {
    fn emit_orchestration_event(&self, event: OrchestrationEvent) -> Result<()>;
}

impl EmitOrchestrationEvent for AppHandle {
    fn emit_orchestration_event(&self, event: OrchestrationEvent) -> Result<()> {
        let emitter = OrchestrationEventEmitter::new(self.clone());
        match event {
            OrchestrationEvent::TaskStatusChanged { orchestration_id, task_id, old_status, new_status, .. } => {
                emitter.emit_task_status_changed(orchestration_id, task_id, old_status, new_status)
            }
            OrchestrationEvent::TaskMessage { orchestration_id, task_id, message_type, message, .. } => {
                emitter.emit_task_message(orchestration_id, task_id, message_type, message)
            }
            OrchestrationEvent::TaskProgress { orchestration_id, task_id, progress, message, .. } => {
                emitter.emit_task_progress(orchestration_id, task_id, progress, message)
            }
            OrchestrationEvent::TaskLog { orchestration_id, task_id, level, message, .. } => {
                emitter.emit_task_log(orchestration_id, task_id, level, message)
            }
            OrchestrationEvent::TaskArtifacts { orchestration_id, task_id, artifacts, .. } => {
                emitter.emit_task_artifacts(orchestration_id, task_id, artifacts)
            }
            OrchestrationEvent::OrchestrationStatusChanged { orchestration_id, status, total_tasks, completed_tasks, failed_tasks, .. } => {
                emitter.emit_orchestration_status_changed(orchestration_id, status, total_tasks, completed_tasks, failed_tasks)
            }
            OrchestrationEvent::TaskAdded { orchestration_id, task, .. } => {
                emitter.emit_task_added(orchestration_id, task)
            }
            OrchestrationEvent::TaskDependencyResolved { orchestration_id, task_id, dependency_id, .. } => {
                emitter.emit_task_dependency_resolved(orchestration_id, task_id, dependency_id)
            }
            OrchestrationEvent::OrchestrationError { orchestration_id, task_id, error, recoverable, .. } => {
                emitter.emit_orchestration_error(orchestration_id, task_id, error, recoverable)
            }
        }
    }
}