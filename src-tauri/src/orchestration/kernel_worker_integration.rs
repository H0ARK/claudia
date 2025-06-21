/// Integration module for connecting OrchestrationKernel with WorkerManager
use anyhow::Result;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::orchestration::{
    agents::{WorkerManager, WorkerType, WorkerConfig},
    kernel::OrchestrationKernel,
    protocol::ProtocolMessage,
    AgentMessage,
};

/// Extended kernel with integrated worker management
pub struct IntegratedOrchestrationKernel {
    kernel: OrchestrationKernel,
    worker_manager: WorkerManager,
}

impl IntegratedOrchestrationKernel {
    /// Create a new integrated orchestration kernel
    pub fn new(
        db_path: std::path::PathBuf,
        claude_binary_path: String,
    ) -> Result<Self> {
        let kernel = OrchestrationKernel::new(db_path, claude_binary_path.clone())?;
        let worker_manager = WorkerManager::new(claude_binary_path);
        
        Ok(Self {
            kernel,
            worker_manager,
        })
    }

    /// Spawn a worker for a task with specific worker type
    pub async fn spawn_typed_worker(
        &self,
        task_id: i64,
        worker_type: WorkerType,
        config: Option<WorkerConfig>,
    ) -> Result<()> {
        let task = self.kernel.get_task(task_id).await?;
        
        // Spawn through worker manager
        let handle = self.worker_manager.spawn_worker(&task, worker_type, config).await?;
        
        // Register message handler to forward to kernel
        let kernel = self.kernel.clone();
        let task_id_clone = task_id;
        
        // Note: In a real implementation, you'd have a proper message handler registration
        // This is a conceptual example
        tokio::spawn(async move {
            // Monitor worker and forward messages to kernel
            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                
                // Check worker status and forward any messages
                // In practice, this would be event-driven rather than polling
            }
        });
        
        Ok(())
    }

    /// Get worker metrics for all tasks in an orchestration
    pub async fn get_orchestration_worker_metrics(
        &self,
        orchestration_id: i64,
    ) -> Result<serde_json::Value> {
        let workers = self.worker_manager.get_active_workers().await;
        
        let mut metrics_by_type = std::collections::HashMap::new();
        let mut total_metrics = crate::orchestration::agents::WorkerMetrics::default();
        
        for worker in workers {
            let metrics = self.worker_manager.get_worker_metrics(&worker).await;
            
            // Aggregate by worker type
            let type_name = worker.worker_type.name();
            let type_metrics = metrics_by_type
                .entry(type_name.to_string())
                .or_insert_with(crate::orchestration::agents::WorkerMetrics::default);
            
            type_metrics.messages_sent += metrics.messages_sent;
            type_metrics.messages_received += metrics.messages_received;
            type_metrics.tasks_completed += metrics.tasks_completed;
            type_metrics.tasks_failed += metrics.tasks_failed;
            type_metrics.error_count += metrics.error_count;
            
            // Update totals
            total_metrics.messages_sent += metrics.messages_sent;
            total_metrics.messages_received += metrics.messages_received;
            total_metrics.tasks_completed += metrics.tasks_completed;
            total_metrics.tasks_failed += metrics.tasks_failed;
            total_metrics.error_count += metrics.error_count;
        }
        
        Ok(serde_json::json!({
            "orchestration_id": orchestration_id,
            "active_workers": workers.len(),
            "total_metrics": total_metrics,
            "metrics_by_type": metrics_by_type,
            "timestamp": chrono::Utc::now(),
        }))
    }

    /// Delegate to kernel methods
    pub async fn create_orchestration(&self, root_goal: String) -> Result<i64> {
        self.kernel.create_orchestration(root_goal).await
    }

    pub async fn schedule_tasks(&self) -> Result<()> {
        self.kernel.schedule_tasks().await
    }

    pub async fn get_orchestration_status(&self, orchestration_id: i64) -> Result<serde_json::Value> {
        self.kernel.get_orchestration_status(orchestration_id).await
    }

    pub async fn cancel_orchestration(&self, orchestration_id: i64) -> Result<()> {
        // First cancel through kernel
        self.kernel.cancel_orchestration(orchestration_id).await?;
        
        // Then terminate all workers for this orchestration
        let workers = self.worker_manager.get_active_workers().await;
        for worker in workers {
            // In practice, you'd check if worker belongs to this orchestration
            let _ = self.worker_manager.terminate_worker(&worker).await;
        }
        
        Ok(())
    }

    /// Get the worker manager for direct access
    pub fn worker_manager(&self) -> &WorkerManager {
        &self.worker_manager
    }

    /// Get the kernel for direct access
    pub fn kernel(&self) -> &OrchestrationKernel {
        &self.kernel
    }
}

/// Example usage of the integrated kernel
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_integrated_kernel_creation() {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");
        
        let integrated = IntegratedOrchestrationKernel::new(
            db_path,
            "/mock/claude".to_string(),
        ).unwrap();
        
        // Test orchestration creation
        let orchestration_id = integrated.create_orchestration(
            "Test orchestration".to_string()
        ).await.unwrap();
        
        assert!(orchestration_id > 0);
        
        // Test worker metrics retrieval
        let metrics = integrated.get_orchestration_worker_metrics(orchestration_id).await.unwrap();
        assert_eq!(metrics["active_workers"], 0);
    }
}