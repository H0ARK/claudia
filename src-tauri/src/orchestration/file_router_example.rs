// Example of how FileRouter integrates with the orchestration system
// This file demonstrates the usage pattern and is not part of the build

use anyhow::Result;
use std::path::PathBuf;
use crate::orchestration::{
    FileRouter, ArtifactId, Task, TaskContext, Artifact, ArtifactType,
    Protocol, JsonProtocol, ProtocolMessage, TaskResult,
};

/// Example: Setting up file routing for an orchestration
async fn setup_task_with_artifacts(
    file_router: &FileRouter,
    task: &Task,
    parent_task_results: Vec<TaskResult>,
) -> Result<TaskContext> {
    // Collect artifacts from parent tasks
    let mut parent_artifacts = Vec::new();
    let mut parent_artifact_ids = Vec::new();
    
    for parent_result in parent_task_results {
        for artifact in parent_result.artifacts {
            // Register each parent artifact with the file router
            let artifact_id = file_router.register_artifact(
                parent_result.task_id,
                artifact.path.clone()
            )?;
            
            parent_artifacts.push(artifact);
            parent_artifact_ids.push(artifact_id);
        }
    }
    
    // Mount parent artifacts for the current task
    let mount_points = file_router.mount_artifacts_for_task(
        task.id,
        parent_artifact_ids
    )?;
    
    // Create MCP configuration
    let mcp_config = file_router.create_mcp_config(task.id, mount_points.clone())?;
    
    // Build task context with mounted paths
    let mut mounted_paths = std::collections::HashMap::new();
    for mount in mount_points {
        mounted_paths.insert(
            mount.artifact_id.as_str().to_string(),
            mount.target
        );
    }
    
    // Create task context
    let context = TaskContext {
        orchestration_id: task.orchestration_id,
        parent_tasks: vec![], // Would be populated with ParentTaskInfo
        mounted_paths,
        environment: mcp_config.environment,
        constraints: vec![
            format!("Output directory: {:?}", file_router.get_task_output_dir(task.id)),
            "All parent artifacts are read-only".to_string(),
        ],
        available_tools: vec![
            "file_read".to_string(),
            "file_write".to_string(),
            "command_execute".to_string(),
        ],
    };
    
    Ok(context)
}

/// Example: Processing task completion and registering new artifacts
async fn handle_task_completion(
    file_router: &FileRouter,
    task_id: i64,
    task_result: &TaskResult,
) -> Result<Vec<ArtifactId>> {
    let mut artifact_ids = Vec::new();
    
    // Register each artifact produced by the task
    for artifact in &task_result.artifacts {
        let artifact_id = file_router.register_artifact(task_id, artifact.path.clone())?;
        artifact_ids.push(artifact_id);
    }
    
    println!("Task {} produced {} artifacts", task_id, artifact_ids.len());
    
    Ok(artifact_ids)
}

/// Example: Full orchestration flow with file routing
async fn orchestration_example() -> Result<()> {
    let project_root = PathBuf::from("/home/user/.claude/projects/my-project");
    let file_router = FileRouter::new(project_root);
    
    // Task 1: Generate some code
    let task1_id = 1;
    let task1_output = file_router.get_task_output_dir(task1_id);
    std::fs::create_dir_all(&task1_output)?;
    
    // Simulate task 1 creating a file
    let code_file = task1_output.join("generated_code.rs");
    std::fs::write(&code_file, "fn main() { println!(\"Hello, world!\"); }")?;
    
    // Register the artifact
    let code_artifact_id = file_router.register_artifact(task1_id, code_file.clone())?;
    
    // Task 2: Test the generated code (depends on Task 1)
    let task2_id = 2;
    
    // Mount Task 1's artifacts for Task 2
    let mounts = file_router.mount_artifacts_for_task(
        task2_id,
        vec![code_artifact_id.clone()]
    )?;
    
    // Create MCP config for Task 2
    let mcp_config = file_router.create_mcp_config(task2_id, mounts)?;
    
    println!("Task 2 MCP servers: {:?}", mcp_config.servers);
    println!("Task 2 environment: {:?}", mcp_config.environment);
    println!("Task 2 sandbox paths: {:?}", mcp_config.sandbox_paths);
    
    // Task 2 would execute with access to Task 1's artifacts
    // The artifacts would be available as read-only mounts
    
    // Cleanup when done
    file_router.cleanup_task_artifacts(task1_id)?;
    file_router.cleanup_task_artifacts(task2_id)?;
    
    Ok(())
}

/// Example: Security validation
fn security_example() -> Result<()> {
    let base_path = PathBuf::from("/home/user/project");
    
    // Valid path
    let valid = PathBuf::from("/home/user/project/subdir/file.txt");
    let validated = FileRouter::validate_path(&base_path, &valid)?;
    println!("Valid path: {:?}", validated);
    
    // Invalid path (traversal attempt)
    let invalid = PathBuf::from("/home/user/project/../../../etc/passwd");
    match FileRouter::validate_path(&base_path, &invalid) {
        Err(e) => println!("Correctly rejected traversal: {}", e),
        Ok(_) => panic!("Should have rejected path traversal!"),
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_orchestration_flow() {
        // This would be a full integration test
        // demonstrating the complete flow
    }
}