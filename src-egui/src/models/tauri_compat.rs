// Tauri-compatible models for the egui frontend
use serde::{Deserialize, Serialize};

/// Agent model that matches the Tauri backend structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TauriAgent {
    pub id: Option<i64>,
    pub name: String,
    pub icon: String,
    pub system_prompt: String,
    pub default_task: Option<String>,
    pub model: String,
    pub sandbox_enabled: bool,
    pub enable_file_read: bool,
    pub enable_file_write: bool,
    pub enable_network: bool,
    pub enable_system_commands: bool,
    pub custom_instructions: Option<String>,
    pub sandbox_profile_id: Option<i64>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

impl Default for TauriAgent {
    fn default() -> Self {
        Self {
            id: None,
            name: String::new(),
            icon: "🤖".to_string(),
            system_prompt: String::new(),
            default_task: None,
            model: "claude-3-opus-20240229".to_string(),
            sandbox_enabled: false,
            enable_file_read: true,
            enable_file_write: false,
            enable_network: false,
            enable_system_commands: false,
            custom_instructions: None,
            sandbox_profile_id: None,
            created_at: None,
            updated_at: None,
        }
    }
}

/// Agent run model that matches the Tauri backend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TauriAgentRun {
    pub id: Option<i64>,
    pub agent_id: i64,
    pub agent_name: String,
    pub agent_icon: String,
    pub task: String,
    pub model: String,
    pub project_path: String,
    pub session_id: String,
    pub status: String,
    pub pid: Option<u32>,
    pub created_at: String,
    pub completed_at: Option<String>,
}

/// Agent run metrics from JSONL
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TauriAgentRunMetrics {
    pub duration_ms: Option<i64>,
    pub total_tokens: Option<i64>,
    pub cost_usd: Option<f64>,
    pub message_count: Option<i64>,
}

/// Combined agent run with metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TauriAgentRunWithMetrics {
    #[serde(flatten)]
    pub run: TauriAgentRun,
    pub metrics: Option<TauriAgentRunMetrics>,
    pub output: Option<String>,
}

// Helper to convert between model formats
impl TauriAgent {
    pub fn display_model_name(&self) -> &str {
        match self.model.as_str() {
            "claude-3-opus-20240229" => "Claude 3 Opus",
            "claude-3-sonnet-20240229" => "Claude 3 Sonnet",
            "claude-3-haiku-20240307" => "Claude 3 Haiku",
            _ => &self.model,
        }
    }
    
    pub fn get_permissions_summary(&self) -> Vec<&'static str> {
        let mut perms = Vec::new();
        if self.enable_file_read { perms.push("📖 Read"); }
        if self.enable_file_write { perms.push("✏️ Write"); }
        if self.enable_network { perms.push("🌐 Network"); }
        if self.enable_system_commands { perms.push("⚡ System"); }
        if self.sandbox_enabled { perms.push("🔒 Sandboxed"); }
        perms
    }
}