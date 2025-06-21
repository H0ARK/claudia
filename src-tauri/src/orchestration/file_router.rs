use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use uuid::Uuid;
use log::{info, warn, error};

use crate::commands::mcp::{MCPServerConfig};

/// Unique identifier for an artifact
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ArtifactId(String);

impl ArtifactId {
    pub fn new() -> Self {
        Self(Uuid::new_v4().to_string())
    }
    
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Information about a mounted path for an agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MountPoint {
    /// Source path (from parent task)
    pub source: PathBuf,
    /// Target path (in agent's view)
    pub target: PathBuf,
    /// Whether this mount is read-only
    pub read_only: bool,
    /// The artifact ID this mount provides access to
    pub artifact_id: ArtifactId,
}

/// Metadata about a registered artifact
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactMetadata {
    pub id: ArtifactId,
    pub task_id: i64,
    pub path: PathBuf,
    pub description: String,
    pub created_at: DateTime<Utc>,
    pub size_bytes: u64,
    pub is_directory: bool,
}

/// MCP configuration for a task's file access
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpConfig {
    /// MCP server configurations for file access
    pub servers: HashMap<String, MCPServerConfig>,
    /// Environment variables to set
    pub environment: HashMap<String, String>,
    /// Sandboxed paths configuration
    pub sandbox_paths: Vec<PathBuf>,
}

/// The FileRouter manages artifact directories and mounting between agents
pub struct FileRouter {
    /// Root directory for the project (~/.claude/projects/<proj>)
    project_root: PathBuf,
    /// Registry of all artifacts
    artifacts: Arc<Mutex<HashMap<ArtifactId, ArtifactMetadata>>>,
    /// Mapping from task ID to its artifacts
    task_artifacts: Arc<Mutex<HashMap<i64, Vec<ArtifactId>>>>,
    /// Active mount points by task ID
    active_mounts: Arc<Mutex<HashMap<i64, Vec<MountPoint>>>>,
}

impl FileRouter {
    /// Create a new FileRouter for a project
    pub fn new(project_path: PathBuf) -> Self {
        Self {
            project_root: project_path,
            artifacts: Arc::new(Mutex::new(HashMap::new())),
            task_artifacts: Arc::new(Mutex::new(HashMap::new())),
            active_mounts: Arc::new(Mutex::new(HashMap::new())),
        }
    }
    
    /// Register an artifact produced by a task
    pub fn register_artifact(&self, task_id: i64, artifact_path: PathBuf) -> Result<ArtifactId> {
        // Validate the path is within the task's output directory
        let task_output_dir = self.get_task_output_dir(task_id);
        let canonical_artifact = artifact_path.canonicalize()
            .context("Failed to canonicalize artifact path")?;
        let canonical_output = task_output_dir.canonicalize()
            .context("Failed to canonicalize output directory")?;
            
        if !canonical_artifact.starts_with(&canonical_output) {
            anyhow::bail!("Artifact path is outside task output directory");
        }
        
        // Get file metadata
        let metadata = fs::metadata(&canonical_artifact)
            .context("Failed to get artifact metadata")?;
        
        // Create artifact metadata
        let artifact_id = ArtifactId::new();
        let artifact_meta = ArtifactMetadata {
            id: artifact_id.clone(),
            task_id,
            path: canonical_artifact,
            description: artifact_path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("Unnamed artifact")
                .to_string(),
            created_at: Utc::now(),
            size_bytes: metadata.len(),
            is_directory: metadata.is_dir(),
        };
        
        // Register the artifact
        {
            let mut artifacts = self.artifacts.lock().unwrap();
            artifacts.insert(artifact_id.clone(), artifact_meta);
        }
        
        // Add to task artifacts mapping
        {
            let mut task_artifacts = self.task_artifacts.lock().unwrap();
            task_artifacts.entry(task_id)
                .or_insert_with(Vec::new)
                .push(artifact_id.clone());
        }
        
        info!("Registered artifact {} for task {}: {:?}", 
              artifact_id.as_str(), task_id, artifact_path);
        
        Ok(artifact_id)
    }
    
    /// Mount artifacts from parent tasks for a child task
    pub fn mount_artifacts_for_task(
        &self, 
        task_id: i64, 
        parent_artifacts: Vec<ArtifactId>
    ) -> Result<Vec<MountPoint>> {
        let mut mount_points = Vec::new();
        let artifacts = self.artifacts.lock().unwrap();
        
        for artifact_id in parent_artifacts {
            let artifact = artifacts.get(&artifact_id)
                .ok_or_else(|| anyhow::anyhow!("Artifact {} not found", artifact_id.as_str()))?;
            
            // Validate the artifact still exists
            if !artifact.path.exists() {
                anyhow::bail!("Artifact {} no longer exists at {:?}", 
                             artifact_id.as_str(), artifact.path);
            }
            
            // Create mount point
            // Target path will be /mnt/artifacts/<artifact_id>/<original_name>
            let target_base = PathBuf::from("/mnt/artifacts").join(artifact_id.as_str());
            let target = if let Some(name) = artifact.path.file_name() {
                target_base.join(name)
            } else {
                target_base
            };
            
            let mount = MountPoint {
                source: artifact.path.clone(),
                target,
                read_only: true,  // Parent artifacts are always read-only
                artifact_id: artifact_id.clone(),
            };
            
            mount_points.push(mount);
        }
        
        // Store active mounts for this task
        {
            let mut active_mounts = self.active_mounts.lock().unwrap();
            active_mounts.insert(task_id, mount_points.clone());
        }
        
        info!("Mounted {} artifacts for task {}", mount_points.len(), task_id);
        
        Ok(mount_points)
    }
    
    /// Get the output directory for a task
    pub fn get_task_output_dir(&self, task_id: i64) -> PathBuf {
        self.project_root
            .join("agents")
            .join(format!("task-{}", task_id))
            .join("output")
    }
    
    /// Clean up artifacts for a task
    pub fn cleanup_task_artifacts(&self, task_id: i64) -> Result<()> {
        // Remove active mounts
        {
            let mut active_mounts = self.active_mounts.lock().unwrap();
            active_mounts.remove(&task_id);
        }
        
        // Get artifacts to clean up
        let artifact_ids = {
            let task_artifacts = self.task_artifacts.lock().unwrap();
            task_artifacts.get(&task_id).cloned().unwrap_or_default()
        };
        
        // Remove artifacts from registry
        {
            let mut artifacts = self.artifacts.lock().unwrap();
            for artifact_id in &artifact_ids {
                artifacts.remove(artifact_id);
            }
        }
        
        // Remove from task artifacts mapping
        {
            let mut task_artifacts = self.task_artifacts.lock().unwrap();
            task_artifacts.remove(&task_id);
        }
        
        // Optionally clean up the filesystem
        let task_output_path = self.get_task_output_dir(task_id);
        let task_dir = task_output_path.parent()
            .ok_or_else(|| anyhow::anyhow!("Invalid task directory"))?;
        
        if task_dir.exists() {
            warn!("Removing task directory: {:?}", task_dir);
            fs::remove_dir_all(task_dir)
                .context("Failed to remove task directory")?;
        }
        
        info!("Cleaned up {} artifacts for task {}", artifact_ids.len(), task_id);
        
        Ok(())
    }
    
    /// Create MCP configuration for a task with mounted artifacts
    pub fn create_mcp_config(&self, task_id: i64, mounts: Vec<MountPoint>) -> Result<McpConfig> {
        let mut servers = HashMap::new();
        let mut environment = HashMap::new();
        let mut sandbox_paths = Vec::new();
        
        // Add the task's output directory as a writable path
        let output_dir = self.get_task_output_dir(task_id);
        
        // Ensure output directory exists
        fs::create_dir_all(&output_dir)
            .context("Failed to create task output directory")?;
        
        sandbox_paths.push(output_dir.clone());
        
        // Create MCP server configuration for file access
        // We'll use the filesystem MCP server for providing file access
        let fs_server = MCPServerConfig {
            command: "npx".to_string(),
            args: vec![
                "-y".to_string(),
                "@modelcontextprotocol/server-filesystem".to_string(),
                output_dir.to_string_lossy().to_string(),
            ],
            env: HashMap::new(),
        };
        
        servers.insert("task-filesystem".to_string(), fs_server);
        
        // For each mount, we need to create appropriate access
        // In practice, this might involve:
        // 1. Creating symlinks in a controlled directory
        // 2. Using bind mounts (requires elevated privileges)
        // 3. Using a custom MCP server that provides controlled access
        
        // For now, we'll document the mount points in environment variables
        // A real implementation would set up actual filesystem mounts
        for (idx, mount) in mounts.iter().enumerate() {
            environment.insert(
                format!("MOUNT_{}_SOURCE", idx),
                mount.source.to_string_lossy().to_string()
            );
            environment.insert(
                format!("MOUNT_{}_TARGET", idx),
                mount.target.to_string_lossy().to_string()
            );
            environment.insert(
                format!("MOUNT_{}_READONLY", idx),
                mount.read_only.to_string()
            );
            
            // Add mounted paths to sandbox paths (read-only)
            if mount.source.exists() {
                sandbox_paths.push(mount.source.clone());
            }
        }
        
        // Set artifact mount count
        environment.insert("MOUNT_COUNT".to_string(), mounts.len().to_string());
        
        // Add task metadata
        environment.insert("TASK_ID".to_string(), task_id.to_string());
        environment.insert("TASK_OUTPUT_DIR".to_string(), output_dir.to_string_lossy().to_string());
        
        Ok(McpConfig {
            servers,
            environment,
            sandbox_paths,
        })
    }
    
    /// Validate a path to prevent directory traversal attacks
    pub fn validate_path(base: &Path, path: &Path) -> Result<PathBuf> {
        let canonical_base = base.canonicalize()
            .context("Failed to canonicalize base path")?;
        let canonical_path = if path.is_absolute() {
            path.canonicalize()
                .context("Failed to canonicalize absolute path")?
        } else {
            base.join(path).canonicalize()
                .context("Failed to canonicalize relative path")?
        };
        
        if !canonical_path.starts_with(&canonical_base) {
            anyhow::bail!("Path traversal detected: {:?} is outside {:?}", 
                         canonical_path, canonical_base);
        }
        
        Ok(canonical_path)
    }
    
    /// Get all artifacts produced by a task
    pub fn get_task_artifacts(&self, task_id: i64) -> Vec<ArtifactMetadata> {
        let task_artifacts = self.task_artifacts.lock().unwrap();
        let artifacts = self.artifacts.lock().unwrap();
        
        task_artifacts.get(&task_id)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| artifacts.get(id).cloned())
                    .collect()
            })
            .unwrap_or_default()
    }
    
    /// Get artifact metadata by ID
    pub fn get_artifact(&self, artifact_id: &ArtifactId) -> Option<ArtifactMetadata> {
        let artifacts = self.artifacts.lock().unwrap();
        artifacts.get(artifact_id).cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    #[test]
    fn test_file_router_creation() {
        let temp_dir = TempDir::new().unwrap();
        let router = FileRouter::new(temp_dir.path().to_path_buf());
        
        let output_dir = router.get_task_output_dir(1);
        assert!(output_dir.to_string_lossy().contains("task-1"));
        assert!(output_dir.to_string_lossy().contains("output"));
    }
    
    #[test]
    fn test_artifact_registration() {
        let temp_dir = TempDir::new().unwrap();
        let router = FileRouter::new(temp_dir.path().to_path_buf());
        
        // Create task output directory
        let task_id = 1;
        let output_dir = router.get_task_output_dir(task_id);
        fs::create_dir_all(&output_dir).unwrap();
        
        // Create a test file
        let artifact_path = output_dir.join("test.txt");
        fs::write(&artifact_path, "test content").unwrap();
        
        // Register the artifact
        let artifact_id = router.register_artifact(task_id, artifact_path.clone()).unwrap();
        
        // Verify artifact is registered
        let artifact = router.get_artifact(&artifact_id).unwrap();
        assert_eq!(artifact.task_id, task_id);
        assert_eq!(artifact.path, artifact_path.canonicalize().unwrap());
        assert!(!artifact.is_directory);
    }
    
    #[test]
    fn test_path_validation() {
        let temp_dir = TempDir::new().unwrap();
        let base = temp_dir.path();
        
        // Valid path within base
        let valid_path = base.join("subdir").join("file.txt");
        fs::create_dir_all(valid_path.parent().unwrap()).unwrap();
        fs::write(&valid_path, "test").unwrap();
        
        let result = FileRouter::validate_path(base, &valid_path);
        assert!(result.is_ok());
        
        // Invalid path (traversal attempt)
        let invalid_path = base.join("..").join("outside");
        let result = FileRouter::validate_path(base, &invalid_path);
        assert!(result.is_err());
    }
    
    #[test]
    fn test_mount_artifacts() {
        let temp_dir = TempDir::new().unwrap();
        let router = FileRouter::new(temp_dir.path().to_path_buf());
        
        // Create parent task artifacts
        let parent_task_id = 1;
        let parent_output = router.get_task_output_dir(parent_task_id);
        fs::create_dir_all(&parent_output).unwrap();
        
        let artifact_path = parent_output.join("data.json");
        fs::write(&artifact_path, r#"{"test": "data"}"#).unwrap();
        
        let artifact_id = router.register_artifact(parent_task_id, artifact_path).unwrap();
        
        // Mount for child task
        let child_task_id = 2;
        let mounts = router.mount_artifacts_for_task(child_task_id, vec![artifact_id.clone()]).unwrap();
        
        assert_eq!(mounts.len(), 1);
        assert!(mounts[0].read_only);
        assert_eq!(mounts[0].artifact_id, artifact_id);
    }
    
    #[test]
    fn test_mcp_config_generation() {
        let temp_dir = TempDir::new().unwrap();
        let router = FileRouter::new(temp_dir.path().to_path_buf());
        
        let task_id = 1;
        let mounts = vec![];
        
        let config = router.create_mcp_config(task_id, mounts).unwrap();
        
        assert!(config.servers.contains_key("task-filesystem"));
        assert_eq!(config.environment.get("TASK_ID").unwrap(), "1");
        assert!(config.sandbox_paths.len() > 0);
    }
}