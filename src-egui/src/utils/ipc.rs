use anyhow::Result;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::Duration;

#[derive(Clone)]
pub struct IpcClient {
    client: Client,
    base_url: String,
}

#[derive(Serialize)]
struct TauriCommand {
    cmd: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    args: Option<Value>,
}

#[derive(Deserialize)]
struct TauriResponse<T> {
    data: Option<T>,
    error: Option<String>,
}

impl IpcClient {
    pub fn new(base_url: &str) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            client,
            base_url: base_url.to_string(),
        }
    }

    pub async fn invoke<T: for<'de> Deserialize<'de>>(
        &self,
        cmd: &str,
        args: Option<Value>,
    ) -> Result<T> {
        let command = TauriCommand {
            cmd: cmd.to_string(),
            args,
        };

        let response = self
            .client
            .post(format!("{}/invoke", self.base_url))
            .json(&command)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "IPC request failed with status: {}",
                response.status()
            ));
        }

        let tauri_response: TauriResponse<T> = response.json().await?;

        if let Some(error) = tauri_response.error {
            return Err(anyhow::anyhow!("Tauri error: {}", error));
        }

        tauri_response
            .data
            .ok_or_else(|| anyhow::anyhow!("No data in response"))
    }

    // Agent-specific methods
    pub async fn list_agents(&self) -> Result<Vec<crate::models::Agent>> {
        self.invoke("list_agents", None).await
    }

    pub async fn create_agent(&self, agent: &crate::models::Agent) -> Result<i64> {
        self.invoke("create_agent", Some(serde_json::to_value(agent)?))
            .await
    }

    pub async fn update_agent(&self, agent: &crate::models::Agent) -> Result<()> {
        self.invoke("update_agent", Some(serde_json::to_value(agent)?))
            .await
    }

    pub async fn delete_agent(&self, id: i64) -> Result<()> {
        self.invoke(
            "delete_agent",
            Some(serde_json::json!({ "id": id })),
        )
        .await
    }

    pub async fn execute_agent(
        &self,
        agent_id: i64,
        project_path: &str,
        task: &str,
    ) -> Result<()> {
        self.invoke(
            "execute_agent",
            Some(serde_json::json!({
                "agentId": agent_id,
                "projectPath": project_path,
                "task": task
            })),
        )
        .await
    }

    // Session methods
    pub async fn get_running_sessions(&self) -> Result<Vec<Value>> {
        self.invoke("list_running_sessions", None).await
    }

    pub async fn kill_session(&self, pid: u32) -> Result<()> {
        self.invoke(
            "kill_agent_session",
            Some(serde_json::json!({ "pid": pid })),
        )
        .await
    }

    // Usage stats
    pub async fn get_usage_stats(&self) -> Result<Value> {
        self.invoke("get_usage_stats", None).await
    }
}