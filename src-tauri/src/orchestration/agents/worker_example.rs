/// Example demonstrating the integration of WorkerManager with OrchestrationKernel
use anyhow::Result;
use std::path::PathBuf;
use uuid::Uuid;
use chrono::Utc;

use crate::orchestration::{
    agents::{WorkerManager, WorkerType, WorkerConfig},
    kernel::OrchestrationKernel,
    Task, TaskStatus, AgentConfig,
};

/// Example: Using WorkerManager with OrchestrationKernel
pub async fn worker_orchestration_example() -> Result<()> {
    // Initialize the orchestration kernel
    let kernel = OrchestrationKernel::new(
        PathBuf::from("./orchestration.db"),
        "/usr/local/bin/claude".to_string(),
    )?;

    // Initialize the worker manager
    let worker_manager = WorkerManager::new("/usr/local/bin/claude".to_string());

    // Create an orchestration with a complex goal
    let orchestration_id = kernel.create_orchestration(
        "Build a REST API with authentication, database integration, and comprehensive tests".to_string()
    ).await?;

    // The planner agent will decompose this into tasks
    // For this example, let's manually create some tasks that would typically be generated

    // Task 1: Design API Schema
    let design_task_id = kernel.add_task(
        orchestration_id,
        "Design API Schema".to_string(),
        "Design RESTful API endpoints and data models".to_string(),
        "Create a comprehensive API design including endpoints, request/response schemas, and data models".to_string(),
        AgentConfig {
            agent_type: "developer".to_string(),
            model: "claude-3-5-sonnet-20241022".to_string(),
            system_prompt: WorkerType::Developer.system_prompt(),
            max_tokens: Some(4096),
            temperature: Some(0.7),
            capabilities: WorkerType::Developer.capabilities(),
            sandbox_profile: None,
        },
        vec![], // No dependencies
    ).await?;

    // Task 2: Implement Database Models
    let db_task_id = kernel.add_task(
        orchestration_id,
        "Implement Database Models".to_string(),
        "Create database schema and ORM models".to_string(),
        "Implement database models based on the API design using SQLAlchemy or similar ORM".to_string(),
        AgentConfig {
            agent_type: "developer".to_string(),
            model: "claude-3-5-sonnet-20241022".to_string(),
            system_prompt: WorkerType::Developer.system_prompt(),
            max_tokens: Some(4096),
            temperature: Some(0.7),
            capabilities: WorkerType::Developer.capabilities(),
            sandbox_profile: None,
        },
        vec![design_task_id], // Depends on design
    ).await?;

    // Task 3: Implement Authentication
    let auth_task_id = kernel.add_task(
        orchestration_id,
        "Implement Authentication".to_string(),
        "Create JWT-based authentication system".to_string(),
        "Implement secure authentication using JWT tokens, including login, logout, and token refresh".to_string(),
        AgentConfig {
            agent_type: "developer".to_string(),
            model: "claude-3-5-sonnet-20241022".to_string(),
            system_prompt: WorkerType::Developer.system_prompt(),
            max_tokens: Some(4096),
            temperature: Some(0.7),
            capabilities: WorkerType::Developer.capabilities(),
            sandbox_profile: None,
        },
        vec![design_task_id], // Depends on design
    ).await?;

    // Task 4: Write Tests
    let test_task_id = kernel.add_task(
        orchestration_id,
        "Write Comprehensive Tests".to_string(),
        "Create unit and integration tests".to_string(),
        "Write comprehensive test suites covering all API endpoints, authentication flows, and edge cases".to_string(),
        AgentConfig {
            agent_type: "tester".to_string(),
            model: "claude-3-5-sonnet-20241022".to_string(),
            system_prompt: WorkerType::Tester.system_prompt(),
            max_tokens: Some(4096),
            temperature: Some(0.7),
            capabilities: WorkerType::Tester.capabilities(),
            sandbox_profile: None,
        },
        vec![db_task_id, auth_task_id], // Depends on implementation
    ).await?;

    // Task 5: Code Review
    let review_task_id = kernel.add_task(
        orchestration_id,
        "Review Implementation".to_string(),
        "Perform security and quality review".to_string(),
        "Review the entire implementation for security vulnerabilities, code quality, and best practices".to_string(),
        AgentConfig {
            agent_type: "reviewer".to_string(),
            model: "claude-3-5-haiku-20241022".to_string(),
            system_prompt: WorkerType::Reviewer.system_prompt(),
            max_tokens: Some(4096),
            temperature: Some(0.5),
            capabilities: WorkerType::Reviewer.capabilities(),
            sandbox_profile: None,
        },
        vec![test_task_id], // Depends on tests being written
    ).await?;

    // Task 6: Generate Documentation
    let docs_task_id = kernel.add_task(
        orchestration_id,
        "Generate Documentation".to_string(),
        "Create API documentation and guides".to_string(),
        "Generate comprehensive documentation including API reference, setup guide, and usage examples".to_string(),
        AgentConfig {
            agent_type: "documentation".to_string(),
            model: "claude-3-5-haiku-20241022".to_string(),
            system_prompt: WorkerType::Documentation.system_prompt(),
            max_tokens: Some(4096),
            temperature: Some(0.7),
            capabilities: WorkerType::Documentation.capabilities(),
            sandbox_profile: None,
        },
        vec![review_task_id], // Depends on review completion
    ).await?;

    // Now let's demonstrate manual worker spawning with custom configurations
    
    // Spawn a custom worker for performance testing
    let perf_config = WorkerConfig {
        worker_type: WorkerType::Custom(1),
        model: Some("claude-3-5-sonnet-20241022".to_string()),
        custom_system_prompt: Some(
            "You are a Performance Testing Agent. Your role is to:\n\
             1. Create performance test scenarios\n\
             2. Implement load testing scripts\n\
             3. Analyze performance metrics\n\
             4. Identify bottlenecks\n\
             5. Suggest optimizations".to_string()
        ),
        additional_capabilities: vec![
            "performance_testing".to_string(),
            "load_testing".to_string(),
            "metrics_analysis".to_string(),
        ],
        ..Default::default()
    };

    // Register the custom worker type
    worker_manager.register_custom_worker_type(1, perf_config.clone()).await?;

    // Example of spawning a worker manually (outside of kernel's automatic spawning)
    let design_task = kernel.get_task(design_task_id).await?;
    let design_worker = worker_manager.spawn_worker(
        &design_task,
        WorkerType::Developer,
        None, // Use default config
    ).await?;

    println!("Spawned design worker: {}", design_worker.id);

    // Monitor worker status
    let status = worker_manager.monitor_worker(&design_worker).await?;
    println!("Worker status: {:?}", status);

    // Get metrics
    let metrics = worker_manager.get_worker_metrics(&design_worker).await;
    println!("Worker metrics: {:?}", metrics);

    // Example of handling multiple concurrent workers
    let active_workers = worker_manager.get_active_workers().await;
    println!("Active workers: {}", active_workers.len());

    // Get workers by type
    let dev_workers = worker_manager.get_workers_by_type(WorkerType::Developer).await;
    println!("Developer workers: {}", dev_workers.len());

    // Example of custom message handling
    // In practice, you'd register a handler that processes worker messages
    // worker_manager.register_message_handler(worker_id, |msg| {
    //     match msg {
    //         ProtocolMessage::TaskProgress(progress) => {
    //             println!("Progress: {}%", progress.progress);
    //         }
    //         _ => {}
    //     }
    // });

    // Start the orchestration execution
    kernel.schedule_tasks().await?;

    // Monitor orchestration progress
    let status = kernel.get_orchestration_status(orchestration_id).await?;
    println!("Orchestration status: {}", serde_json::to_string_pretty(&status)?);

    Ok(())
}

/// Example: Using WorkerManager for parallel task execution
pub async fn parallel_worker_example() -> Result<()> {
    let worker_manager = WorkerManager::new("/usr/local/bin/claude".to_string());

    // Create multiple tasks that can run in parallel
    let tasks = vec![
        ("Analyze codebase structure", WorkerType::Developer),
        ("Security audit", WorkerType::Reviewer),
        ("Performance profiling", WorkerType::Custom(1)),
        ("Generate test cases", WorkerType::Tester),
    ];

    let mut handles = Vec::new();

    // Spawn workers for parallel execution
    for (goal, worker_type) in tasks {
        let task = Task {
            id: Uuid::new_v4().as_u128() as i64,
            orchestration_id: 1,
            name: goal.to_string(),
            description: format!("Parallel task: {}", goal),
            goal: goal.to_string(),
            agent_config: AgentConfig {
                agent_type: worker_type.name().to_string(),
                model: worker_type.default_model().to_string(),
                system_prompt: worker_type.system_prompt(),
                max_tokens: Some(4096),
                temperature: Some(0.7),
                capabilities: worker_type.capabilities(),
                sandbox_profile: None,
            },
            status: TaskStatus::Ready,
            created_at: Utc::now(),
            started_at: None,
            completed_at: None,
            output: None,
            error: None,
            retry_count: 0,
            max_retries: 3,
            depends_on: Vec::new(),
        };

        let handle = worker_manager.spawn_worker(&task, worker_type, None).await?;
        handles.push(handle);
    }

    // Monitor all workers
    for handle in &handles {
        let status = worker_manager.monitor_worker(handle).await?;
        println!("Worker {} status: {:?}", handle.id, status);
    }

    // Get global metrics
    let global_metrics = worker_manager.get_global_metrics().await;
    println!("Global metrics: {:?}", global_metrics);

    Ok(())
}

/// Example: Worker failure and recovery
pub async fn worker_recovery_example() -> Result<()> {
    let worker_manager = WorkerManager::new("/usr/local/bin/claude".to_string());

    // Create a task that might fail
    let task = Task {
        id: 1,
        orchestration_id: 1,
        name: "Complex Analysis".to_string(),
        description: "Analyze complex system architecture".to_string(),
        goal: "Perform deep analysis of system architecture and identify improvements".to_string(),
        agent_config: AgentConfig {
            agent_type: "developer".to_string(),
            model: "claude-3-5-sonnet-20241022".to_string(),
            system_prompt: WorkerType::Developer.system_prompt(),
            max_tokens: Some(4096),
            temperature: Some(0.7),
            capabilities: WorkerType::Developer.capabilities(),
            sandbox_profile: None,
        },
        status: TaskStatus::Ready,
        created_at: Utc::now(),
        started_at: None,
        completed_at: None,
        output: None,
        error: None,
        retry_count: 0,
        max_retries: 3,
        depends_on: Vec::new(),
    };

    // Spawn initial worker
    let handle = worker_manager.spawn_worker(&task, WorkerType::Developer, None).await?;
    println!("Spawned worker: {}", handle.id);

    // Simulate monitoring for failure
    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

    let status = worker_manager.monitor_worker(&handle).await?;
    match status {
        WorkerStatus::Failed(error) => {
            println!("Worker failed: {}", error);
            
            // Attempt to restart the worker
            println!("Attempting to restart worker...");
            let new_handle = worker_manager.restart_worker(&handle).await?;
            println!("Worker restarted with new ID: {}", new_handle.id);
        }
        _ => {
            println!("Worker is running normally");
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_worker_orchestration_setup() {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");
        
        let kernel = OrchestrationKernel::new(db_path, "/mock/claude".to_string()).unwrap();
        let worker_manager = WorkerManager::new("/mock/claude".to_string());

        // Test orchestration creation
        let orchestration_id = kernel.create_orchestration("Test goal".to_string()).await.unwrap();
        assert!(orchestration_id > 0);

        // Test worker manager initialization
        let workers = worker_manager.get_active_workers().await;
        assert!(workers.is_empty());
    }
}