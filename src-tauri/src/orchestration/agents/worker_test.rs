#[cfg(test)]
mod tests {
    use super::super::*;
    use std::sync::Arc;
    use tokio::sync::Mutex;
    use tempfile::tempdir;

    /// Mock Claude binary for testing
    async fn create_mock_claude_script() -> String {
        let script = r#"#!/bin/bash
echo '{"type": "task_started", "task_id": 1, "timestamp": "2024-01-01T00:00:00Z"}'
sleep 1
echo '{"type": "task_progress", "task_id": 1, "progress": 50.0, "message": "Processing...", "timestamp": "2024-01-01T00:00:01Z"}'
sleep 1
echo '{"type": "task_result", "task_id": 1, "status": "completed", "artifacts": [], "summary": "Task completed successfully", "timestamp": "2024-01-01T00:00:02Z"}'
"#;
        
        let temp_dir = tempdir().unwrap();
        let script_path = temp_dir.path().join("mock_claude.sh");
        std::fs::write(&script_path, script).unwrap();
        
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&script_path, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        
        script_path.to_string_lossy().to_string()
    }

    #[tokio::test]
    async fn test_worker_spawn_and_monitor() {
        let mock_claude = create_mock_claude_script().await;
        let manager = WorkerManager::new(mock_claude);

        let task = Task {
            id: 1,
            orchestration_id: 1,
            name: "Test Task".to_string(),
            description: "Test description".to_string(),
            goal: "Test goal".to_string(),
            agent_config: AgentConfig {
                agent_type: "developer".to_string(),
                model: "test-model".to_string(),
                system_prompt: "Test prompt".to_string(),
                max_tokens: Some(100),
                temperature: Some(0.5),
                capabilities: vec!["test".to_string()],
                sandbox_profile: None,
            },
            status: TaskStatus::Ready,
            created_at: chrono::Utc::now(),
            started_at: None,
            completed_at: None,
            output: None,
            error: None,
            retry_count: 0,
            max_retries: 3,
            depends_on: Vec::new(),
        };

        // Spawn worker
        let handle = manager.spawn_worker(&task, WorkerType::Developer, None).await.unwrap();
        assert_eq!(handle.worker_type, WorkerType::Developer);
        assert_eq!(handle.task_id, 1);

        // Check initial status
        let status = manager.monitor_worker(&handle).await.unwrap();
        match status {
            WorkerStatus::Starting | WorkerStatus::Running => {},
            _ => panic!("Unexpected initial status: {:?}", status),
        }

        // Wait a bit for processing
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

        // Check active workers
        let active = manager.get_active_workers().await;
        assert!(!active.is_empty());

        // Check workers by type
        let dev_workers = manager.get_workers_by_type(WorkerType::Developer).await;
        assert_eq!(dev_workers.len(), 1);

        // Check metrics
        let metrics = manager.get_worker_metrics(&handle).await;
        assert!(metrics.messages_received > 0);

        // Terminate worker
        manager.terminate_worker(&handle).await.unwrap();

        // Verify termination
        let active_after = manager.get_active_workers().await;
        assert!(active_after.is_empty());
    }

    #[tokio::test]
    async fn test_multiple_worker_types() {
        let manager = WorkerManager::new("/mock/claude".to_string());

        // Test all worker types have proper configuration
        let worker_types = vec![
            WorkerType::Developer,
            WorkerType::Tester,
            WorkerType::Reviewer,
            WorkerType::Documentation,
            WorkerType::Custom(1),
        ];

        for wt in worker_types {
            assert!(!wt.name().is_empty());
            assert!(!wt.default_model().is_empty());
            assert!(!wt.system_prompt().is_empty());
            assert!(!wt.capabilities().is_empty());
        }
    }

    #[tokio::test]
    async fn test_custom_worker_registration() {
        let manager = WorkerManager::new("/mock/claude".to_string());

        let custom_config = WorkerConfig {
            worker_type: WorkerType::Custom(42),
            model: Some("custom-model".to_string()),
            custom_system_prompt: Some("Custom system prompt".to_string()),
            additional_capabilities: vec!["custom_capability".to_string()],
            ..Default::default()
        };

        manager.register_custom_worker_type(42, custom_config.clone()).await.unwrap();

        // Verify registration by attempting to spawn (won't actually run without valid binary)
        let task = Task::default();
        let result = manager.spawn_worker(&task, WorkerType::Custom(42), None).await;
        
        // The spawn will fail due to invalid binary, but config should be found
        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("spawn") || error_msg.contains("process"));
    }

    #[tokio::test]
    async fn test_worker_config_builder() {
        let config = WorkerConfig {
            worker_type: WorkerType::Developer,
            model: Some("claude-3-opus-20240229".to_string()),
            max_tokens: Some(8192),
            temperature: Some(0.3),
            custom_system_prompt: Some("Custom developer prompt".to_string()),
            additional_capabilities: vec!["advanced_analysis".to_string()],
            sandbox_profile: Some("restricted".to_string()),
            environment_vars: vec![
                ("ENV_VAR_1".to_string(), "value1".to_string()),
                ("ENV_VAR_2".to_string(), "value2".to_string()),
            ].into_iter().collect(),
            working_directory: Some(std::path::PathBuf::from("/tmp/work")),
        };

        assert_eq!(config.worker_type, WorkerType::Developer);
        assert_eq!(config.model.as_ref().unwrap(), "claude-3-opus-20240229");
        assert_eq!(config.max_tokens, Some(8192));
        assert_eq!(config.temperature, Some(0.3));
        assert!(config.custom_system_prompt.is_some());
        assert_eq!(config.additional_capabilities.len(), 1);
        assert_eq!(config.sandbox_profile.as_ref().unwrap(), "restricted");
        assert_eq!(config.environment_vars.len(), 2);
        assert!(config.working_directory.is_some());
    }

    #[tokio::test]
    async fn test_worker_metrics() {
        let metrics = WorkerMetrics {
            messages_sent: 10,
            messages_received: 15,
            tasks_completed: 3,
            tasks_failed: 1,
            total_runtime_seconds: 120,
            error_count: 2,
        };

        assert_eq!(metrics.messages_sent, 10);
        assert_eq!(metrics.messages_received, 15);
        assert_eq!(metrics.tasks_completed, 3);
        assert_eq!(metrics.tasks_failed, 1);
        assert_eq!(metrics.total_runtime_seconds, 120);
        assert_eq!(metrics.error_count, 2);

        // Test default
        let default_metrics = WorkerMetrics::default();
        assert_eq!(default_metrics.messages_sent, 0);
        assert_eq!(default_metrics.messages_received, 0);
    }

    #[tokio::test]
    async fn test_worker_status_transitions() {
        use std::sync::Arc;
        use tokio::sync::RwLock;

        let status = Arc::new(RwLock::new(WorkerStatus::Starting));

        // Test status transitions
        *status.write().await = WorkerStatus::Running;
        assert!(matches!(*status.read().await, WorkerStatus::Running));

        *status.write().await = WorkerStatus::Busy;
        assert!(matches!(*status.read().await, WorkerStatus::Busy));

        *status.write().await = WorkerStatus::Idle;
        assert!(matches!(*status.read().await, WorkerStatus::Idle));

        *status.write().await = WorkerStatus::Failed("Test error".to_string());
        match &*status.read().await {
            WorkerStatus::Failed(msg) => assert_eq!(msg, "Test error"),
            _ => panic!("Expected Failed status"),
        }

        *status.write().await = WorkerStatus::Stopped;
        assert!(matches!(*status.read().await, WorkerStatus::Stopped));
    }

    #[tokio::test]
    async fn test_system_prompts() {
        // Ensure all system prompts are non-empty and contain expected keywords
        assert!(DEVELOPER_SYSTEM_PROMPT.contains("Developer Agent"));
        assert!(DEVELOPER_SYSTEM_PROMPT.contains("implementation"));

        assert!(TESTER_SYSTEM_PROMPT.contains("Tester Agent"));
        assert!(TESTER_SYSTEM_PROMPT.contains("testing"));

        assert!(REVIEWER_SYSTEM_PROMPT.contains("Code Reviewer Agent"));
        assert!(REVIEWER_SYSTEM_PROMPT.contains("review"));

        assert!(DOCUMENTATION_SYSTEM_PROMPT.contains("Documentation Agent"));
        assert!(DOCUMENTATION_SYSTEM_PROMPT.contains("documentation"));

        assert!(CUSTOM_SYSTEM_PROMPT.contains("Custom Agent"));
    }
}