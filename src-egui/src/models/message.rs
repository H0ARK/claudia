use crate::models::agent::AgentId;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MessageId(pub Uuid);

impl MessageId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: MessageId,
    pub from: AgentId,
    pub to: MessageTarget,
    pub message_type: MessageType,
    pub content: MessageContent,
    pub priority: Priority,
    pub timestamp: DateTime<Utc>,
    pub in_reply_to: Option<MessageId>,
    pub requires_response: bool,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageTarget {
    Agent(AgentId),
    Role(String), // Role name
    Broadcast,
    Group(Vec<AgentId>),
    Orchestrator,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageType {
    TaskAssignment,
    StatusUpdate,
    ResultDelivery,
    CollaborationRequest,
    ReviewRequest,
    ApprovalRequest,
    Information,
    Error,
    Warning,
    Query,
    Response,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageContent {
    pub text: String,
    pub structured_data: Option<serde_json::Value>,
    pub attachments: Vec<Attachment>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attachment {
    pub id: Uuid,
    pub name: String,
    pub mime_type: String,
    pub size_bytes: u64,
    pub data: AttachmentData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AttachmentData {
    Inline(Vec<u8>),
    Reference(String), // Path or URL
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum Priority {
    Low = 0,
    Normal = 1,
    High = 2,
    Urgent = 3,
    Critical = 4,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub requirements: HashMap<String, String>,
    pub inputs: Vec<TaskInput>,
    pub expected_outputs: Vec<String>,
    pub deadline: Option<DateTime<Utc>>,
    pub dependencies: Vec<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskInput {
    pub name: String,
    pub value: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResult {
    pub task_id: Uuid,
    pub status: TaskStatus,
    pub outputs: HashMap<String, serde_json::Value>,
    pub logs: Vec<String>,
    pub metrics: TaskMetrics,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum TaskStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
    Cancelled,
    Blocked,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskMetrics {
    pub start_time: DateTime<Utc>,
    pub end_time: Option<DateTime<Utc>>,
    pub tokens_used: u64,
    pub cost_usd: f64,
    pub retry_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollaborationRequest {
    pub request_type: CollaborationType,
    pub context: String,
    pub required_expertise: Vec<String>,
    pub urgency: Priority,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CollaborationType {
    PairProgramming,
    CodeReview,
    DesignDiscussion,
    ProblemSolving,
    KnowledgeSharing,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewRequest {
    pub artifact_type: String,
    pub artifact_location: String,
    pub review_criteria: Vec<String>,
    pub severity_threshold: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalRequest {
    pub request_id: Uuid,
    pub subject: String,
    pub details: String,
    pub options: Vec<ApprovalOption>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalOption {
    pub id: String,
    pub label: String,
    pub description: Option<String>,
}

impl Message {
    pub fn new(
        from: AgentId,
        to: MessageTarget,
        message_type: MessageType,
        content: MessageContent,
    ) -> Self {
        Self {
            id: MessageId::new(),
            from,
            to,
            message_type,
            content,
            priority: Priority::Normal,
            timestamp: Utc::now(),
            in_reply_to: None,
            requires_response: false,
            metadata: HashMap::new(),
        }
    }

    pub fn with_priority(mut self, priority: Priority) -> Self {
        self.priority = priority;
        self
    }

    pub fn requires_response(mut self) -> Self {
        self.requires_response = true;
        self
    }

    pub fn in_reply_to(mut self, message_id: MessageId) -> Self {
        self.in_reply_to = Some(message_id);
        self
    }

    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }
}