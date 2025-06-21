pub mod agent;
pub mod message;
pub mod workflow;
pub mod tauri_compat;

pub use agent::{Agent, AgentId, AgentRole, AgentStatus, ModelType, Permissions};
pub use message::{Message, MessageId, MessageType, MessageContent, MessageTarget, Task, TaskResult, TaskStatus, Priority};
pub use workflow::{Workflow, WorkflowId, WorkflowNode, WorkflowNodeType, WorkflowTask};
pub use tauri_compat::{TauriAgent, TauriAgentRun, TauriAgentRunMetrics, TauriAgentRunWithMetrics};