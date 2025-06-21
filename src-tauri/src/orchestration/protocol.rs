use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;

use super::{AgentMessage, Task, TaskStatus};

/// Task request sent from kernel to agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskRequest {
    pub task_id: i64,
    pub goal: String,
    pub context: TaskContext,
    pub timestamp: DateTime<Utc>,
}

/// Task result sent from agent to kernel
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResult {
    pub task_id: i64,
    pub status: TaskStatus,
    pub artifacts: Vec<Artifact>,
    pub summary: String,
    pub timestamp: DateTime<Utc>,
}

/// Task progress update sent from agent to kernel
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskProgress {
    pub task_id: i64,
    pub progress: f32,  // 0.0 to 100.0
    pub message: String,
    pub timestamp: DateTime<Utc>,
}

/// Task log message sent from agent to kernel
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskLog {
    pub task_id: i64,
    pub level: LogLevel,
    pub message: String,
    pub timestamp: DateTime<Utc>,
}

/// Log level for task messages
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Debug,
    Info,
    Warning,
    Error,
}

impl std::fmt::Display for LogLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LogLevel::Debug => write!(f, "DEBUG"),
            LogLevel::Info => write!(f, "INFO"),
            LogLevel::Warning => write!(f, "WARN"),
            LogLevel::Error => write!(f, "ERROR"),
        }
    }
}

/// Context provided to an agent for task execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskContext {
    pub orchestration_id: i64,
    pub parent_tasks: Vec<ParentTaskInfo>,
    pub mounted_paths: HashMap<String, PathBuf>,
    pub environment: HashMap<String, String>,
    pub constraints: Vec<String>,
    pub available_tools: Vec<String>,
}

/// Information about a parent task that this task depends on
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParentTaskInfo {
    pub task_id: i64,
    pub name: String,
    pub summary: String,
    pub artifacts: Vec<Artifact>,
}

/// An artifact produced by a task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Artifact {
    pub name: String,
    pub artifact_type: ArtifactType,
    pub path: PathBuf,
    pub description: String,
    pub metadata: HashMap<String, Value>,
}

/// Type of artifact produced
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactType {
    File,
    Directory,
    CodeModule,
    Documentation,
    TestReport,
    BuildArtifact,
    Configuration,
    Other,
}

/// Unified message envelope for all agent-kernel communication
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ProtocolMessage {
    // Kernel -> Agent
    TaskRequest(TaskRequest),
    TaskCancel { task_id: i64, reason: String },
    
    // Agent -> Kernel
    TaskStarted { task_id: i64, timestamp: DateTime<Utc> },
    TaskProgress(TaskProgress),
    TaskLog(TaskLog),
    TaskResult(TaskResult),
    
    // Bidirectional
    Heartbeat { task_id: i64, timestamp: DateTime<Utc> },
    Error { task_id: i64, error: String, recoverable: bool },
}

/// Protocol handler trait for message parsing and formatting
pub trait Protocol {
    /// Parse agent output into protocol messages
    fn parse_agent_output(output: &str) -> Result<ProtocolMessage>;
    
    /// Format a task request for sending to an agent
    fn format_task_request(task: &Task) -> Result<String>;
    
    /// Validate a protocol message
    fn validate_message(message: &ProtocolMessage) -> Result<()>;
}

/// Default protocol implementation
pub struct JsonProtocol;

impl Protocol for JsonProtocol {
    fn parse_agent_output(output: &str) -> Result<ProtocolMessage> {
        // Try to parse as JSON first
        if let Ok(json_msg) = serde_json::from_str::<ProtocolMessage>(output) {
            return Ok(json_msg);
        }
        
        // Try to parse as Claude Code streaming JSON format
        if let Ok(claude_msg) = serde_json::from_str::<Value>(output) {
            return parse_claude_streaming_json(claude_msg);
        }
        
        // Fallback: try to extract structured data from human-readable output
        parse_human_readable_output(output)
    }
    
    fn format_task_request(task: &Task) -> Result<String> {
        let context = build_task_context(task)?;
        
        let request = TaskRequest {
            task_id: task.id,
            goal: task.goal.clone(),
            context,
            timestamp: Utc::now(),
        };
        
        let message = ProtocolMessage::TaskRequest(request);
        
        serde_json::to_string_pretty(&message)
            .context("Failed to serialize task request")
    }
    
    fn validate_message(message: &ProtocolMessage) -> Result<()> {
        match message {
            ProtocolMessage::TaskRequest(req) => {
                if req.task_id <= 0 {
                    anyhow::bail!("Invalid task_id: must be positive");
                }
                if req.goal.trim().is_empty() {
                    anyhow::bail!("Task goal cannot be empty");
                }
            }
            ProtocolMessage::TaskProgress(progress) => {
                if progress.progress < 0.0 || progress.progress > 100.0 {
                    anyhow::bail!("Progress must be between 0 and 100");
                }
            }
            ProtocolMessage::TaskResult(result) => {
                if result.summary.trim().is_empty() {
                    anyhow::bail!("Task result summary cannot be empty");
                }
            }
            _ => {}
        }
        Ok(())
    }
}

/// Parse Claude Code streaming JSON format
fn parse_claude_streaming_json(value: Value) -> Result<ProtocolMessage> {
    let msg_type = value.get("type")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing message type"))?;
    
    match msg_type {
        "completion" => {
            let content = value.get("content")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("Missing content"))?;
            
            // Extract task_id from environment or content
            let task_id = extract_task_id_from_content(content)
                .or_else(|| value.get("task_id").and_then(|v| v.as_i64()))
                .unwrap_or(0);
            
            Ok(ProtocolMessage::TaskResult(TaskResult {
                task_id,
                status: TaskStatus::Completed,
                artifacts: extract_artifacts_from_content(content),
                summary: content.to_string(),
                timestamp: Utc::now(),
            }))
        }
        "error" => {
            let error = value.get("error")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("Missing error message"))?;
            
            let task_id = value.get("task_id")
                .and_then(|v| v.as_i64())
                .unwrap_or(0);
            
            Ok(ProtocolMessage::Error {
                task_id,
                error: error.to_string(),
                recoverable: true,
            })
        }
        "progress" => {
            let task_id = value.get("task_id")
                .and_then(|v| v.as_i64())
                .ok_or_else(|| anyhow::anyhow!("Missing task_id"))?;
            
            let progress = value.get("progress")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0) as f32;
            
            let message = value.get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            
            Ok(ProtocolMessage::TaskProgress(TaskProgress {
                task_id,
                progress,
                message,
                timestamp: Utc::now(),
            }))
        }
        _ => anyhow::bail!("Unknown message type: {}", msg_type),
    }
}

/// Parse human-readable output and extract structured information
fn parse_human_readable_output(output: &str) -> Result<ProtocolMessage> {
    // Look for common patterns in agent output
    let _lines: Vec<&str> = output.lines().collect();
    
    // Check for task completion patterns
    if output.contains("Task completed") || output.contains("Finished") {
        let task_id = extract_task_id_from_content(output).unwrap_or(0);
        
        return Ok(ProtocolMessage::TaskResult(TaskResult {
            task_id,
            status: TaskStatus::Completed,
            artifacts: extract_artifacts_from_content(output),
            summary: extract_summary_from_output(output),
            timestamp: Utc::now(),
        }));
    }
    
    // Check for progress updates
    if let Some(progress) = extract_progress_from_output(output) {
        let task_id = extract_task_id_from_content(output).unwrap_or(0);
        
        return Ok(ProtocolMessage::TaskProgress(TaskProgress {
            task_id,
            progress,
            message: output.to_string(),
            timestamp: Utc::now(),
        }));
    }
    
    // Check for error patterns
    if output.contains("Error:") || output.contains("Failed") {
        let task_id = extract_task_id_from_content(output).unwrap_or(0);
        
        return Ok(ProtocolMessage::Error {
            task_id,
            error: output.to_string(),
            recoverable: !output.contains("Fatal"),
        });
    }
    
    // Default to log message
    let level = if output.contains("ERROR") {
        LogLevel::Error
    } else if output.contains("WARN") {
        LogLevel::Warning
    } else if output.contains("DEBUG") {
        LogLevel::Debug
    } else {
        LogLevel::Info
    };
    
    Ok(ProtocolMessage::TaskLog(TaskLog {
        task_id: extract_task_id_from_content(output).unwrap_or(0),
        level,
        message: output.to_string(),
        timestamp: Utc::now(),
    }))
}

/// Build task context from task information
fn build_task_context(task: &Task) -> Result<TaskContext> {
    // TODO: Implement parent task lookup and artifact mounting
    Ok(TaskContext {
        orchestration_id: task.orchestration_id,
        parent_tasks: Vec::new(),
        mounted_paths: HashMap::new(),
        environment: HashMap::new(),
        constraints: Vec::new(),
        available_tools: vec![
            "file_read".to_string(),
            "file_write".to_string(),
            "command_execute".to_string(),
            "web_search".to_string(),
        ],
    })
}

/// Extract task ID from content
fn extract_task_id_from_content(content: &str) -> Option<i64> {
    // Look for patterns like "Task #123" or "task_id: 123"
    if let Some(cap) = regex::Regex::new(r"(?i)task[# _]+(\d+)")
        .ok()?
        .captures(content)
    {
        return cap.get(1)?.as_str().parse().ok();
    }
    None
}

/// Extract artifacts from content
fn extract_artifacts_from_content(content: &str) -> Vec<Artifact> {
    let mut artifacts = Vec::new();
    
    // Look for file paths mentioned in the output
    let path_regex = regex::Regex::new(r"(?:Created|Modified|Generated|Wrote)\s+(?:file\s+)?([/\w\-_.]+\.\w+)").unwrap();
    
    for cap in path_regex.captures_iter(content) {
        if let Some(path_match) = cap.get(1) {
            let path = PathBuf::from(path_match.as_str());
            let name = path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown")
                .to_string();
            
            let artifact_type = match path.extension().and_then(|e| e.to_str()) {
                Some("rs") | Some("py") | Some("js") | Some("ts") => ArtifactType::CodeModule,
                Some("md") | Some("txt") | Some("rst") => ArtifactType::Documentation,
                Some("json") | Some("yaml") | Some("toml") => ArtifactType::Configuration,
                _ => ArtifactType::File,
            };
            
            artifacts.push(Artifact {
                name,
                artifact_type,
                path,
                description: String::new(),
                metadata: HashMap::new(),
            });
        }
    }
    
    artifacts
}

/// Extract summary from output
fn extract_summary_from_output(output: &str) -> String {
    // Try to find a summary section
    if let Some(summary_start) = output.find("Summary:") {
        let summary = &output[summary_start + 8..];
        if let Some(end) = summary.find('\n') {
            return summary[..end].trim().to_string();
        }
    }
    
    // Fallback: use first non-empty line
    output.lines()
        .find(|line| !line.trim().is_empty())
        .unwrap_or("")
        .to_string()
}

/// Extract progress percentage from output
fn extract_progress_from_output(output: &str) -> Option<f32> {
    // Look for patterns like "50%" or "Progress: 50"
    let progress_regex = regex::Regex::new(r"(?i)(?:progress:?\s*)?(\d+(?:\.\d+)?)\s*%").ok()?;
    
    if let Some(cap) = progress_regex.captures(output) {
        return cap.get(1)?.as_str().parse().ok();
    }
    None
}

/// Convert AgentMessage to ProtocolMessage
impl From<AgentMessage> for ProtocolMessage {
    fn from(msg: AgentMessage) -> Self {
        match msg {
            AgentMessage::TaskStarted { task_id, timestamp } => {
                ProtocolMessage::TaskStarted { task_id, timestamp }
            }
            AgentMessage::TaskProgress { task_id, progress, message } => {
                ProtocolMessage::TaskProgress(TaskProgress {
                    task_id,
                    progress,
                    message,
                    timestamp: Utc::now(),
                })
            }
            AgentMessage::TaskCompleted { task_id, success, output, timestamp } => {
                ProtocolMessage::TaskResult(TaskResult {
                    task_id,
                    status: if success { TaskStatus::Completed } else { TaskStatus::Failed },
                    artifacts: extract_artifacts_from_content(&output),
                    summary: output,
                    timestamp,
                })
            }
            AgentMessage::TaskFailed { task_id, error, recoverable, timestamp: _ } => {
                ProtocolMessage::Error {
                    task_id,
                    error,
                    recoverable,
                }
            }
            _ => {
                // Handle other message types as needed
                ProtocolMessage::TaskLog(TaskLog {
                    task_id: 0,
                    level: LogLevel::Info,
                    message: format!("{:?}", msg),
                    timestamp: Utc::now(),
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_json_protocol_message() {
        let json = r#"{
            "type": "task_progress",
            "task_id": 123,
            "progress": 50.0,
            "message": "Processing files",
            "timestamp": "2024-01-01T00:00:00Z"
        }"#;
        
        let result = JsonProtocol::parse_agent_output(json);
        assert!(result.is_ok());
        
        if let Ok(ProtocolMessage::TaskProgress(progress)) = result {
            assert_eq!(progress.task_id, 123);
            assert_eq!(progress.progress, 50.0);
            assert_eq!(progress.message, "Processing files");
        } else {
            panic!("Expected TaskProgress message");
        }
    }
    
    #[test]
    fn test_parse_human_readable_output() {
        let output = "Task #42 completed successfully\nCreated file src/main.rs";
        
        let result = JsonProtocol::parse_agent_output(output);
        assert!(result.is_ok());
        
        if let Ok(ProtocolMessage::TaskResult(result)) = result {
            assert_eq!(result.task_id, 42);
            assert_eq!(result.status, TaskStatus::Completed);
            assert!(!result.artifacts.is_empty());
        } else {
            panic!("Expected TaskResult message");
        }
    }
    
    #[test]
    fn test_validate_message() {
        let valid_progress = ProtocolMessage::TaskProgress(TaskProgress {
            task_id: 1,
            progress: 50.0,
            message: "Working".to_string(),
            timestamp: Utc::now(),
        });
        
        assert!(JsonProtocol::validate_message(&valid_progress).is_ok());
        
        let invalid_progress = ProtocolMessage::TaskProgress(TaskProgress {
            task_id: 1,
            progress: 150.0, // Invalid: > 100
            message: "Working".to_string(),
            timestamp: Utc::now(),
        });
        
        assert!(JsonProtocol::validate_message(&invalid_progress).is_err());
    }
    
    #[test]
    fn test_extract_artifacts() {
        let content = "Created file src/lib.rs\nModified config.json\nGenerated docs/README.md";
        let artifacts = extract_artifacts_from_content(content);
        
        assert_eq!(artifacts.len(), 3);
        assert_eq!(artifacts[0].name, "lib.rs");
        assert_eq!(artifacts[0].artifact_type, ArtifactType::CodeModule);
        assert_eq!(artifacts[1].name, "config.json");
        assert_eq!(artifacts[1].artifact_type, ArtifactType::Configuration);
        assert_eq!(artifacts[2].name, "README.md");
        assert_eq!(artifacts[2].artifact_type, ArtifactType::Documentation);
    }
}