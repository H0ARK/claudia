use anyhow::Result;
use std::collections::HashMap;

use crate::orchestration::agents::planner::{
    AgentType, PlannerAgent, PlannerContext, ResourceConstraints,
};

/// Example usage of the PlannerAgent
pub async fn example_planner_usage() -> Result<()> {
    // Create a new planner agent
    let planner = PlannerAgent::new();

    // Define available agent types
    let available_agents = vec![
        AgentType {
            name: "analyst".to_string(),
            capabilities: vec![
                "text_analysis".to_string(),
                "requirement_extraction".to_string(),
                "documentation".to_string(),
            ],
            model: "claude-3-opus-20240229".to_string(),
            description: "Analyzes text and extracts structured information".to_string(),
        },
        AgentType {
            name: "developer".to_string(),
            capabilities: vec![
                "code_generation".to_string(),
                "testing".to_string(),
                "debugging".to_string(),
                "refactoring".to_string(),
            ],
            model: "claude-3-opus-20240229".to_string(),
            description: "Develops code and implements solutions".to_string(),
        },
        AgentType {
            name: "reviewer".to_string(),
            capabilities: vec![
                "code_review".to_string(),
                "quality_assurance".to_string(),
                "best_practices".to_string(),
            ],
            model: "claude-3-haiku-20240307".to_string(),
            description: "Reviews code and ensures quality standards".to_string(),
        },
        AgentType {
            name: "tester".to_string(),
            capabilities: vec![
                "test_generation".to_string(),
                "test_execution".to_string(),
                "bug_finding".to_string(),
            ],
            model: "claude-3-sonnet-20240229".to_string(),
            description: "Creates and executes tests".to_string(),
        },
    ];

    // Define resource constraints
    let resource_constraints = ResourceConstraints {
        max_concurrent_tasks: 3,
        max_duration_seconds: Some(3600), // 1 hour
        available_memory_gb: Some(16.0),
        available_cpu_cores: Some(8),
    };

    // Create planner context
    let context = PlannerContext {
        available_agents,
        resource_constraints,
        existing_tasks: vec![], // No existing tasks in this example
        additional_context: HashMap::from([
            ("project_type".to_string(), serde_json::json!("web_application")),
            ("language".to_string(), serde_json::json!("rust")),
            ("framework".to_string(), serde_json::json!("tauri")),
        ]),
    };

    // Example 1: Plan a feature implementation
    println!("Example 1: Planning a feature implementation");
    let goal1 = "Implement a user authentication system with email/password login, \
                 session management, and password reset functionality for a Tauri application";
    
    let plan1 = planner.plan_orchestration(goal1, &context).await?;
    print_plan_summary(&plan1);

    // Example 2: Plan a bug fix workflow
    println!("\nExample 2: Planning a bug fix workflow");
    let goal2 = "Fix a memory leak in the file upload component that causes the application \
                 to crash after processing large files (>100MB)";
    
    let plan2 = planner.plan_orchestration(goal2, &context).await?;
    print_plan_summary(&plan2);

    // Example 3: Plan a refactoring task
    println!("\nExample 3: Planning a refactoring task");
    let goal3 = "Refactor the monolithic API module into smaller, domain-specific modules \
                 with proper separation of concerns and improved testability";
    
    let plan3 = planner.plan_orchestration(goal3, &context).await?;
    print_plan_summary(&plan3);

    // Example 4: Validate and optimize a manually created plan
    println!("\nExample 4: Validating and optimizing a plan");
    let mut manual_plan = create_manual_plan();
    
    // Validate the plan
    match planner.validate_plan(&manual_plan) {
        Ok(_) => println!("Manual plan is valid"),
        Err(e) => println!("Manual plan validation failed: {}", e),
    }

    // Optimize the plan
    planner.optimize_plan(&mut manual_plan)?;
    println!("Plan optimized successfully");

    Ok(())
}

/// Helper function to print a plan summary
fn print_plan_summary(plan: &crate::orchestration::TaskPlan) {
    println!("Goal: {}", plan.goal);
    println!("Total tasks: {}", plan.tasks.len());
    println!("Estimated duration: {} seconds", 
             plan.estimated_duration_seconds.unwrap_or(0));
    
    println!("\nTasks:");
    for task in &plan.tasks {
        println!("  - {} ({}): {}", task.id, task.suggested_agent, task.name);
        if !task.expected_inputs.is_empty() {
            println!("    Inputs: {}", 
                     task.expected_inputs.iter()
                         .map(|i| format!("{} from {}", i.name, i.from_task_id))
                         .collect::<Vec<_>>()
                         .join(", "));
        }
        if !task.expected_outputs.is_empty() {
            println!("    Outputs: {}", 
                     task.expected_outputs.iter()
                         .map(|o| o.name.clone())
                         .collect::<Vec<_>>()
                         .join(", "));
        }
    }
    
    println!("\nDependencies:");
    for (task_id, deps) in &plan.dependencies {
        if !deps.is_empty() {
            println!("  {} depends on: {}", task_id, deps.join(", "));
        }
    }
    
    println!("\nSuccess criteria:");
    for criterion in &plan.success_criteria {
        println!("  - {}", criterion);
    }
    
    if !plan.metadata.risks_and_mitigations.is_empty() {
        println!("\nRisks and mitigations:");
        for risk in &plan.metadata.risks_and_mitigations {
            println!("  - Risk: {} (Impact: {}, Likelihood: {})", 
                     risk.risk, risk.impact, risk.likelihood);
            println!("    Mitigation: {}", risk.mitigation);
        }
    }
}

/// Create a manual plan for testing
fn create_manual_plan() -> crate::orchestration::TaskPlan {
    use crate::orchestration::{
        PlannedTask, ExpectedInput, ExpectedOutput, 
        ResourceRequirements, PlanMetadata, RiskMitigation,
    };
    use chrono::Utc;
    use std::collections::HashMap;

    crate::orchestration::TaskPlan {
        goal: "Create a REST API for user management".to_string(),
        tasks: vec![
            PlannedTask {
                id: "design_api".to_string(),
                name: "Design API Schema".to_string(),
                description: "Design the REST API endpoints and data models".to_string(),
                goal: "Create OpenAPI specification for user management endpoints".to_string(),
                suggested_agent: "analyst".to_string(),
                expected_inputs: vec![],
                expected_outputs: vec![
                    ExpectedOutput {
                        name: "api_spec".to_string(),
                        output_type: "file".to_string(),
                        description: "OpenAPI 3.0 specification".to_string(),
                    }
                ],
                estimated_duration_seconds: Some(600),
                priority: 10,
                retriable: true,
                success_criteria: vec!["Complete API specification with all endpoints defined".to_string()],
            },
            PlannedTask {
                id: "implement_models".to_string(),
                name: "Implement Data Models".to_string(),
                description: "Create database models and DTOs".to_string(),
                goal: "Implement User model and related data structures".to_string(),
                suggested_agent: "developer".to_string(),
                expected_inputs: vec![
                    ExpectedInput {
                        name: "api_spec".to_string(),
                        input_type: "file".to_string(),
                        from_task_id: "design_api".to_string(),
                        description: "API specification to implement".to_string(),
                    }
                ],
                expected_outputs: vec![
                    ExpectedOutput {
                        name: "models".to_string(),
                        output_type: "file".to_string(),
                        description: "Database models and DTOs".to_string(),
                    }
                ],
                estimated_duration_seconds: Some(900),
                priority: 8,
                retriable: true,
                success_criteria: vec!["All models implemented with proper validation".to_string()],
            },
            PlannedTask {
                id: "implement_endpoints".to_string(),
                name: "Implement API Endpoints".to_string(),
                description: "Create REST API endpoints for user management".to_string(),
                goal: "Implement CRUD endpoints for user management".to_string(),
                suggested_agent: "developer".to_string(),
                expected_inputs: vec![
                    ExpectedInput {
                        name: "api_spec".to_string(),
                        input_type: "file".to_string(),
                        from_task_id: "design_api".to_string(),
                        description: "API specification to implement".to_string(),
                    },
                    ExpectedInput {
                        name: "models".to_string(),
                        input_type: "file".to_string(),
                        from_task_id: "implement_models".to_string(),
                        description: "Data models to use".to_string(),
                    }
                ],
                expected_outputs: vec![
                    ExpectedOutput {
                        name: "api_implementation".to_string(),
                        output_type: "file".to_string(),
                        description: "Implemented API endpoints".to_string(),
                    }
                ],
                estimated_duration_seconds: Some(1200),
                priority: 7,
                retriable: true,
                success_criteria: vec!["All endpoints implemented and functional".to_string()],
            },
            PlannedTask {
                id: "write_tests".to_string(),
                name: "Write Tests".to_string(),
                description: "Create unit and integration tests".to_string(),
                goal: "Write comprehensive tests for the API".to_string(),
                suggested_agent: "tester".to_string(),
                expected_inputs: vec![
                    ExpectedInput {
                        name: "api_implementation".to_string(),
                        input_type: "file".to_string(),
                        from_task_id: "implement_endpoints".to_string(),
                        description: "API implementation to test".to_string(),
                    }
                ],
                expected_outputs: vec![
                    ExpectedOutput {
                        name: "test_suite".to_string(),
                        output_type: "file".to_string(),
                        description: "Test suite for the API".to_string(),
                    }
                ],
                estimated_duration_seconds: Some(800),
                priority: 6,
                retriable: true,
                success_criteria: vec!["All endpoints have test coverage".to_string()],
            },
        ],
        dependencies: HashMap::from([
            ("implement_models".to_string(), vec!["design_api".to_string()]),
            ("implement_endpoints".to_string(), vec!["design_api".to_string(), "implement_models".to_string()]),
            ("write_tests".to_string(), vec!["implement_endpoints".to_string()]),
        ]),
        estimated_duration_seconds: Some(3500),
        resource_requirements: ResourceRequirements {
            peak_memory_gb: Some(4.0),
            peak_cpu_cores: Some(2),
            total_compute_hours: Some(1.0),
        },
        success_criteria: vec![
            "API fully implemented with all CRUD operations".to_string(),
            "All tests passing".to_string(),
            "API documentation complete".to_string(),
        ],
        metadata: PlanMetadata {
            created_at: Utc::now(),
            planner_version: "1.0.0".to_string(),
            confidence_score: 0.85,
            alternative_approaches: vec![
                "Use GraphQL instead of REST".to_string(),
                "Implement using serverless functions".to_string(),
            ],
            risks_and_mitigations: vec![
                RiskMitigation {
                    risk: "Database schema changes during development".to_string(),
                    impact: "medium".to_string(),
                    likelihood: "medium".to_string(),
                    mitigation: "Use database migrations and version control".to_string(),
                },
                RiskMitigation {
                    risk: "API specification incomplete".to_string(),
                    impact: "high".to_string(),
                    likelihood: "low".to_string(),
                    mitigation: "Review specification with stakeholders before implementation".to_string(),
                },
            ],
        },
    }
}