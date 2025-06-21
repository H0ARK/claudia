/// Example usage of the Orchestration Kernel
/// 
/// This module demonstrates how to use the orchestration kernel to:
/// 1. Create an orchestration with a root goal
/// 2. Add tasks with dependencies
/// 3. Schedule and execute tasks
/// 4. Handle agent messages
use super::kernel::{OrchestrationKernel, AgentConfig, TaskStatus};
use anyhow::Result;
use std::path::PathBuf;

/// Example: Create a web application development orchestration
pub async fn example_web_app_orchestration() -> Result<()> {
    // Initialize kernel
    let kernel = OrchestrationKernel::new(
        PathBuf::from("./orchestration.db"),
        "claude".to_string(),
    )?;
    
    // Create orchestration with root goal
    let orchestration_id = kernel.create_orchestration(
        "Build a TODO list web application with React frontend and Node.js backend".to_string()
    ).await?;
    
    println!("Created orchestration: {}", orchestration_id);
    
    // The initial decomposition task will be created automatically
    // Let's schedule it to run
    kernel.schedule_tasks().await?;
    
    // In a real scenario, the decomposition task would create subtasks
    // For this example, let's manually add some tasks
    
    // Task 1: Design the application architecture
    let architecture_task_id = kernel.add_task(
        orchestration_id,
        "Design Architecture".to_string(),
        "Design the overall architecture for the TODO list application".to_string(),
        "Create a detailed architecture design including: database schema, API endpoints, component structure, and deployment strategy".to_string(),
        AgentConfig {
            agent_type: "architect".to_string(),
            model: "claude-3-5-sonnet-20241022".to_string(),
            system_prompt: "You are a software architect specializing in web applications.".to_string(),
            max_tokens: Some(4096),
            temperature: Some(0.7),
            capabilities: vec!["architecture".to_string(), "system-design".to_string()],
            sandbox_profile: None,
        },
        vec![], // No dependencies
    ).await?;
    
    // Task 2: Set up backend project
    let backend_setup_task_id = kernel.add_task(
        orchestration_id,
        "Setup Backend".to_string(),
        "Initialize Node.js backend project with Express".to_string(),
        "Create a Node.js project with Express, set up folder structure, install dependencies, and create basic server configuration".to_string(),
        AgentConfig {
            agent_type: "developer".to_string(),
            model: "claude-3-5-sonnet-20241022".to_string(),
            system_prompt: "You are a backend developer specializing in Node.js and Express.".to_string(),
            max_tokens: Some(4096),
            temperature: Some(0.5),
            capabilities: vec!["nodejs".to_string(), "express".to_string(), "backend".to_string()],
            sandbox_profile: Some("development".to_string()),
        },
        vec![architecture_task_id], // Depends on architecture design
    ).await?;
    
    // Task 3: Set up frontend project
    let frontend_setup_task_id = kernel.add_task(
        orchestration_id,
        "Setup Frontend".to_string(),
        "Initialize React frontend project".to_string(),
        "Create a React project using Vite, set up folder structure, install dependencies including state management and UI libraries".to_string(),
        AgentConfig {
            agent_type: "developer".to_string(),
            model: "claude-3-5-sonnet-20241022".to_string(),
            system_prompt: "You are a frontend developer specializing in React and modern web development.".to_string(),
            max_tokens: Some(4096),
            temperature: Some(0.5),
            capabilities: vec!["react".to_string(), "frontend".to_string(), "javascript".to_string()],
            sandbox_profile: Some("development".to_string()),
        },
        vec![architecture_task_id], // Depends on architecture design
    ).await?;
    
    // Task 4: Implement backend API
    let backend_api_task_id = kernel.add_task(
        orchestration_id,
        "Implement Backend API".to_string(),
        "Implement TODO list API endpoints".to_string(),
        "Implement RESTful API endpoints for CRUD operations on TODO items, including authentication middleware".to_string(),
        AgentConfig {
            agent_type: "developer".to_string(),
            model: "claude-3-5-sonnet-20241022".to_string(),
            system_prompt: "You are a backend developer implementing RESTful APIs.".to_string(),
            max_tokens: Some(8192),
            temperature: Some(0.5),
            capabilities: vec!["nodejs".to_string(), "api".to_string(), "database".to_string()],
            sandbox_profile: Some("development".to_string()),
        },
        vec![backend_setup_task_id], // Depends on backend setup
    ).await?;
    
    // Task 5: Implement frontend components
    let frontend_components_task_id = kernel.add_task(
        orchestration_id,
        "Implement Frontend Components".to_string(),
        "Create React components for TODO list functionality".to_string(),
        "Implement React components for displaying, adding, editing, and deleting TODO items with proper state management".to_string(),
        AgentConfig {
            agent_type: "developer".to_string(),
            model: "claude-3-5-sonnet-20241022".to_string(),
            system_prompt: "You are a frontend developer implementing React components.".to_string(),
            max_tokens: Some(8192),
            temperature: Some(0.5),
            capabilities: vec!["react".to_string(), "components".to_string(), "ui".to_string()],
            sandbox_profile: Some("development".to_string()),
        },
        vec![frontend_setup_task_id], // Depends on frontend setup
    ).await?;
    
    // Task 6: Integration
    let integration_task_id = kernel.add_task(
        orchestration_id,
        "Frontend-Backend Integration".to_string(),
        "Connect frontend to backend API".to_string(),
        "Integrate the React frontend with the Node.js backend API, implement API calls, handle responses, and manage application state".to_string(),
        AgentConfig {
            agent_type: "developer".to_string(),
            model: "claude-3-5-sonnet-20241022".to_string(),
            system_prompt: "You are a full-stack developer specializing in frontend-backend integration.".to_string(),
            max_tokens: Some(4096),
            temperature: Some(0.5),
            capabilities: vec!["integration".to_string(), "api".to_string(), "fullstack".to_string()],
            sandbox_profile: Some("development".to_string()),
        },
        vec![backend_api_task_id, frontend_components_task_id], // Depends on both API and components
    ).await?;
    
    // Task 7: Testing
    let testing_task_id = kernel.add_task(
        orchestration_id,
        "Write Tests".to_string(),
        "Create comprehensive test suite".to_string(),
        "Write unit tests for backend API endpoints and frontend components, plus integration tests for the full application".to_string(),
        AgentConfig {
            agent_type: "tester".to_string(),
            model: "claude-3-5-sonnet-20241022".to_string(),
            system_prompt: "You are a QA engineer specializing in testing web applications.".to_string(),
            max_tokens: Some(4096),
            temperature: Some(0.5),
            capabilities: vec!["testing".to_string(), "jest".to_string(), "cypress".to_string()],
            sandbox_profile: Some("testing".to_string()),
        },
        vec![integration_task_id], // Depends on integration
    ).await?;
    
    // Task 8: Documentation
    let documentation_task_id = kernel.add_task(
        orchestration_id,
        "Write Documentation".to_string(),
        "Create project documentation".to_string(),
        "Write comprehensive documentation including README, API documentation, setup instructions, and deployment guide".to_string(),
        AgentConfig {
            agent_type: "documentation".to_string(),
            model: "claude-3-5-sonnet-20241022".to_string(),
            system_prompt: "You are a technical writer specializing in software documentation.".to_string(),
            max_tokens: Some(4096),
            temperature: Some(0.7),
            capabilities: vec!["documentation".to_string(), "markdown".to_string()],
            sandbox_profile: None,
        },
        vec![testing_task_id], // Depends on testing completion
    ).await?;
    
    // Schedule all tasks
    kernel.schedule_tasks().await?;
    
    // Get orchestration status
    let status = kernel.get_orchestration_status(orchestration_id).await?;
    println!("Orchestration status: {}", serde_json::to_string_pretty(&status)?);
    
    Ok(())
}

/// Example: Handling parallel tasks
pub async fn example_parallel_processing() -> Result<()> {
    let kernel = OrchestrationKernel::new(
        PathBuf::from("./orchestration.db"),
        "claude".to_string(),
    )?;
    
    // Create orchestration for data processing
    let orchestration_id = kernel.create_orchestration(
        "Process and analyze multiple datasets in parallel".to_string()
    ).await?;
    
    // Add data fetching tasks that can run in parallel
    let datasets = vec!["sales_data", "customer_data", "inventory_data", "marketing_data"];
    let mut fetch_task_ids = Vec::new();
    
    for dataset in datasets {
        let task_id = kernel.add_task(
            orchestration_id,
            format!("Fetch {}", dataset),
            format!("Fetch and validate {} from source", dataset),
            format!("Retrieve {} from the database, validate data integrity, and prepare for analysis", dataset),
            AgentConfig {
                agent_type: "data_fetcher".to_string(),
                model: "claude-3-5-sonnet-20241022".to_string(),
                system_prompt: "You are a data engineer specializing in data extraction and validation.".to_string(),
                max_tokens: Some(2048),
                temperature: Some(0.3),
                capabilities: vec!["data".to_string(), "sql".to_string()],
                sandbox_profile: Some("data_access".to_string()),
            },
            vec![], // No dependencies - can run in parallel
        ).await?;
        fetch_task_ids.push(task_id);
    }
    
    // Add analysis task that depends on all fetch tasks
    let analysis_task_id = kernel.add_task(
        orchestration_id,
        "Analyze Combined Data".to_string(),
        "Perform comprehensive analysis on all datasets".to_string(),
        "Combine all datasets and perform cross-dataset analysis, generate insights and visualizations".to_string(),
        AgentConfig {
            agent_type: "data_analyst".to_string(),
            model: "claude-3-5-sonnet-20241022".to_string(),
            system_prompt: "You are a data analyst specializing in business intelligence and data visualization.".to_string(),
            max_tokens: Some(8192),
            temperature: Some(0.7),
            capabilities: vec!["analysis".to_string(), "visualization".to_string(), "python".to_string()],
            sandbox_profile: Some("data_analysis".to_string()),
        },
        fetch_task_ids, // Depends on all fetch tasks
    ).await?;
    
    // Schedule tasks - fetch tasks will run in parallel
    kernel.schedule_tasks().await?;
    
    Ok(())
}

/// Example: Error handling and task retry
pub async fn example_error_handling() -> Result<()> {
    let kernel = OrchestrationKernel::new(
        PathBuf::from("./orchestration.db"),
        "claude".to_string(),
    )?;
    
    let orchestration_id = kernel.create_orchestration(
        "Deploy application with error recovery".to_string()
    ).await?;
    
    // Add deployment task with retry configuration
    let deploy_task_id = kernel.add_task(
        orchestration_id,
        "Deploy to Production".to_string(),
        "Deploy application to production environment".to_string(),
        "Deploy the application to production servers, run health checks, and verify deployment success".to_string(),
        AgentConfig {
            agent_type: "devops".to_string(),
            model: "claude-3-5-sonnet-20241022".to_string(),
            system_prompt: "You are a DevOps engineer specializing in deployment automation.".to_string(),
            max_tokens: Some(4096),
            temperature: Some(0.3),
            capabilities: vec!["deployment".to_string(), "kubernetes".to_string(), "monitoring".to_string()],
            sandbox_profile: Some("deployment".to_string()),
        },
        vec![],
    ).await?;
    
    // Simulate task failure
    kernel.handle_agent_message(
        deploy_task_id,
        super::AgentMessage::TaskFailed {
            task_id: deploy_task_id,
            error: "Health check failed: service not responding on port 8080".to_string(),
            recoverable: true,
            timestamp: chrono::Utc::now(),
        }
    ).await?;
    
    // The kernel will automatically retry the task based on retry configuration
    
    Ok(())
}