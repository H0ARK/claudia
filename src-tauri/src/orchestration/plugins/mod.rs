use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use libloading::{Library, Symbol};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tokio::sync::RwLock;

use crate::orchestration::{
    Artifact, ArtifactType, LogLevel, ProtocolMessage, TaskContext, TaskLog, TaskProgress, TaskResult,
};

/// Plugin trait that all orchestration plugins must implement
pub trait Plugin: Send + Sync {
    /// Get the plugin name
    fn name(&self) -> &str;
    
    /// Get the plugin version
    fn version(&self) -> &str;
    
    /// Get the plugin description
    fn description(&self) -> &str;
    
    /// Get supported agent types this plugin can handle
    fn supported_agent_types(&self) -> Vec<String>;
    
    /// Initialize the plugin with configuration
    fn initialize(&mut self, config: PluginConfig) -> Result<()>;
    
    /// Execute a task
    fn execute_task(
        &self,
        task_id: i64,
        goal: String,
        context: TaskContext,
        callback: Box<dyn PluginCallback>,
    ) -> Result<()>;
    
    /// Cleanup resources
    fn cleanup(&mut self) -> Result<()>;
}

/// Callback interface for plugins to communicate with the kernel
pub trait PluginCallback: Send + Sync {
    /// Send a progress update
    fn on_progress(&self, task_id: i64, progress: f32, message: String) -> Result<()>;
    
    /// Send a log message
    fn on_log(&self, task_id: i64, level: LogLevel, message: String) -> Result<()>;
    
    /// Send the final result
    fn on_complete(&self, task_id: i64, result: TaskResult) -> Result<()>;
    
    /// Report an error
    fn on_error(&self, task_id: i64, error: String, recoverable: bool) -> Result<()>;
}

/// Plugin configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginConfig {
    pub name: String,
    pub version: String,
    pub settings: HashMap<String, serde_json::Value>,
}

/// Plugin metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginMetadata {
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
    pub license: String,
    pub supported_agent_types: Vec<String>,
    pub capabilities: Vec<String>,
}

/// Plugin instance wrapper
struct PluginInstance {
    library: Library,
    plugin: Box<dyn Plugin>,
    metadata: PluginMetadata,
}

unsafe impl Send for PluginInstance {}
unsafe impl Sync for PluginInstance {}

/// Plugin registry for managing loaded plugins
#[derive(Clone)]
pub struct PluginRegistry {
    plugins: Arc<RwLock<HashMap<String, PluginInstance>>>,
    plugin_dir: PathBuf,
}

impl PluginRegistry {
    /// Create a new plugin registry
    pub fn new(plugin_dir: PathBuf) -> Self {
        Self {
            plugins: Arc::new(RwLock::new(HashMap::new())),
            plugin_dir,
        }
    }
    
    /// Load a plugin from a dynamic library
    pub async fn load_plugin(&self, path: &Path) -> Result<String> {
        // Ensure the path is within the plugin directory (security check)
        let canonical_path = path.canonicalize()
            .context("Failed to canonicalize plugin path")?;
        let canonical_plugin_dir = self.plugin_dir.canonicalize()
            .context("Failed to canonicalize plugin directory")?;
        
        if !canonical_path.starts_with(&canonical_plugin_dir) {
            anyhow::bail!("Plugin path is outside the plugin directory");
        }
        
        // Load the library
        let library = unsafe {
            Library::new(&canonical_path)
                .context("Failed to load plugin library")?
        };
        
        // Get the plugin creation function
        let create_plugin: Symbol<fn() -> Box<dyn Plugin>> = unsafe {
            library.get(b"create_plugin")
                .context("Plugin must export 'create_plugin' function")?
        };
        
        // Create the plugin instance
        let mut plugin = create_plugin();
        
        // Get plugin metadata
        let name = plugin.name().to_string();
        let metadata = PluginMetadata {
            name: name.clone(),
            version: plugin.version().to_string(),
            description: plugin.description().to_string(),
            author: String::new(), // TODO: Add author to plugin trait
            license: String::new(), // TODO: Add license to plugin trait
            supported_agent_types: plugin.supported_agent_types(),
            capabilities: Vec::new(), // TODO: Add capabilities to plugin trait
        };
        
        // Initialize with default config
        let config = PluginConfig {
            name: name.clone(),
            version: metadata.version.clone(),
            settings: HashMap::new(),
        };
        plugin.initialize(config)?;
        
        // Store the plugin
        let instance = PluginInstance {
            library,
            plugin,
            metadata,
        };
        
        let mut plugins = self.plugins.write().await;
        plugins.insert(name.clone(), instance);
        
        Ok(name)
    }
    
    /// Unload a plugin
    pub async fn unload_plugin(&self, name: &str) -> Result<()> {
        let mut plugins = self.plugins.write().await;
        
        if let Some(mut instance) = plugins.remove(name) {
            instance.plugin.cleanup()?;
            // Library will be dropped and unloaded automatically
            Ok(())
        } else {
            anyhow::bail!("Plugin '{}' not found", name);
        }
    }
    
    /// Get a list of loaded plugins
    pub async fn list_plugins(&self) -> Vec<PluginMetadata> {
        let plugins = self.plugins.read().await;
        plugins.values()
            .map(|instance| instance.metadata.clone())
            .collect()
    }
    
    /// Execute a task using a plugin
    pub async fn execute_task(
        &self,
        plugin_name: &str,
        task_id: i64,
        goal: String,
        context: TaskContext,
        callback: Box<dyn PluginCallback>,
    ) -> Result<()> {
        let plugins = self.plugins.read().await;
        
        let instance = plugins.get(plugin_name)
            .ok_or_else(|| anyhow::anyhow!("Plugin '{}' not found", plugin_name))?;
        
        instance.plugin.execute_task(task_id, goal, context, callback)
    }
    
    /// Find a plugin that supports a given agent type
    pub async fn find_plugin_for_agent_type(&self, agent_type: &str) -> Option<String> {
        let plugins = self.plugins.read().await;
        
        for (name, instance) in plugins.iter() {
            if instance.metadata.supported_agent_types.contains(&agent_type.to_string()) {
                return Some(name.clone());
            }
        }
        
        None
    }
    
    /// Scan plugin directory and load all plugins
    pub async fn scan_and_load_plugins(&self) -> Result<Vec<String>> {
        let mut loaded = Vec::new();
        
        if !self.plugin_dir.exists() {
            std::fs::create_dir_all(&self.plugin_dir)
                .context("Failed to create plugin directory")?;
        }
        
        let entries = std::fs::read_dir(&self.plugin_dir)
            .context("Failed to read plugin directory")?;
        
        for entry in entries {
            let entry = entry?;
            let path = entry.path();
            
            if path.is_file() {
                let extension = path.extension()
                    .and_then(|ext| ext.to_str());
                
                // Check for dynamic library extensions
                #[cfg(target_os = "linux")]
                let is_library = extension == Some("so");
                
                #[cfg(target_os = "macos")]
                let is_library = extension == Some("dylib");
                
                #[cfg(target_os = "windows")]
                let is_library = extension == Some("dll");
                
                if is_library {
                    match self.load_plugin(&path).await {
                        Ok(name) => {
                            log::info!("Loaded plugin: {}", name);
                            loaded.push(name);
                        }
                        Err(e) => {
                            log::error!("Failed to load plugin {:?}: {}", path, e);
                        }
                    }
                }
            }
        }
        
        Ok(loaded)
    }
}

/// Default callback implementation that sends messages to the orchestration kernel
pub struct KernelCallback {
    sender: tokio::sync::mpsc::Sender<ProtocolMessage>,
}

impl KernelCallback {
    pub fn new(sender: tokio::sync::mpsc::Sender<ProtocolMessage>) -> Self {
        Self { sender }
    }
}

impl PluginCallback for KernelCallback {
    fn on_progress(&self, task_id: i64, progress: f32, message: String) -> Result<()> {
        let msg = ProtocolMessage::TaskProgress(TaskProgress {
            task_id,
            progress,
            message,
            timestamp: Utc::now(),
        });
        
        self.sender.blocking_send(msg)
            .map_err(|e| anyhow::anyhow!("Failed to send progress: {}", e))
    }
    
    fn on_log(&self, task_id: i64, level: LogLevel, message: String) -> Result<()> {
        let msg = ProtocolMessage::TaskLog(TaskLog {
            task_id,
            level,
            message,
            timestamp: Utc::now(),
        });
        
        self.sender.blocking_send(msg)
            .map_err(|e| anyhow::anyhow!("Failed to send log: {}", e))
    }
    
    fn on_complete(&self, task_id: i64, result: TaskResult) -> Result<()> {
        let msg = ProtocolMessage::TaskResult(result);
        
        self.sender.blocking_send(msg)
            .map_err(|e| anyhow::anyhow!("Failed to send result: {}", e))
    }
    
    fn on_error(&self, task_id: i64, error: String, recoverable: bool) -> Result<()> {
        let msg = ProtocolMessage::Error {
            task_id,
            error,
            recoverable,
        };
        
        self.sender.blocking_send(msg)
            .map_err(|e| anyhow::anyhow!("Failed to send error: {}", e))
    }
}

/// Example plugin implementation for testing
pub struct ExamplePlugin {
    name: String,
    version: String,
    initialized: bool,
}

impl ExamplePlugin {
    pub fn new() -> Self {
        Self {
            name: "example".to_string(),
            version: "0.1.0".to_string(),
            initialized: false,
        }
    }
}

impl Plugin for ExamplePlugin {
    fn name(&self) -> &str {
        &self.name
    }
    
    fn version(&self) -> &str {
        &self.version
    }
    
    fn description(&self) -> &str {
        "Example plugin for demonstration purposes"
    }
    
    fn supported_agent_types(&self) -> Vec<String> {
        vec!["example".to_string(), "test".to_string()]
    }
    
    fn initialize(&mut self, _config: PluginConfig) -> Result<()> {
        self.initialized = true;
        Ok(())
    }
    
    fn execute_task(
        &self,
        task_id: i64,
        goal: String,
        context: TaskContext,
        callback: Box<dyn PluginCallback>,
    ) -> Result<()> {
        if !self.initialized {
            anyhow::bail!("Plugin not initialized");
        }
        
        // Simulate task execution in a separate thread
        std::thread::spawn(move || {
            // Send initial progress
            callback.on_log(task_id, LogLevel::Info, format!("Starting task: {}", goal)).ok();
            callback.on_progress(task_id, 0.0, "Initializing...".to_string()).ok();
            
            // Simulate work
            for i in 1..=10 {
                std::thread::sleep(std::time::Duration::from_millis(100));
                let progress = (i as f32) * 10.0;
                callback.on_progress(
                    task_id,
                    progress,
                    format!("Processing step {}/10", i),
                ).ok();
                callback.on_log(
                    task_id,
                    LogLevel::Debug,
                    format!("Completed step {}", i),
                ).ok();
            }
            
            // Create a dummy artifact
            let artifact = Artifact {
                name: "example_output.txt".to_string(),
                artifact_type: ArtifactType::File,
                path: PathBuf::from("/tmp/example_output.txt"),
                description: "Example output file".to_string(),
                metadata: HashMap::new(),
            };
            
            // Send completion
            let result = TaskResult {
                task_id,
                status: crate::orchestration::TaskStatus::Completed,
                artifacts: vec![artifact],
                summary: format!("Successfully completed task: {}", goal),
                timestamp: Utc::now(),
            };
            
            callback.on_complete(task_id, result).ok();
        });
        
        Ok(())
    }
    
    fn cleanup(&mut self) -> Result<()> {
        self.initialized = false;
        Ok(())
    }
}

/// Export function for creating the example plugin
#[no_mangle]
pub extern "C" fn create_plugin() -> Box<dyn Plugin> {
    Box::new(ExamplePlugin::new())
}

/// Plugin sandbox for secure execution
pub struct PluginSandbox {
    /// Resource limits
    memory_limit: usize,
    cpu_time_limit: std::time::Duration,
    /// Allowed paths for file access
    allowed_paths: Vec<PathBuf>,
    /// Network access control
    allow_network: bool,
}

impl PluginSandbox {
    pub fn new() -> Self {
        Self {
            memory_limit: 100 * 1024 * 1024, // 100MB
            cpu_time_limit: std::time::Duration::from_secs(300), // 5 minutes
            allowed_paths: vec![],
            allow_network: false,
        }
    }
    
    pub fn with_memory_limit(mut self, limit: usize) -> Self {
        self.memory_limit = limit;
        self
    }
    
    pub fn with_cpu_time_limit(mut self, limit: std::time::Duration) -> Self {
        self.cpu_time_limit = limit;
        self
    }
    
    pub fn allow_path(mut self, path: PathBuf) -> Self {
        self.allowed_paths.push(path);
        self
    }
    
    pub fn allow_network(mut self, allow: bool) -> Self {
        self.allow_network = allow;
        self
    }
    
    /// Execute a function within the sandbox
    pub fn execute<F, T>(&self, f: F) -> Result<T>
    where
        F: FnOnce() -> Result<T> + Send + 'static,
        T: Send + 'static,
    {
        // TODO: Implement actual sandboxing using OS-specific features
        // For now, just execute the function directly
        f()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    #[tokio::test]
    async fn test_plugin_registry() {
        let temp_dir = TempDir::new().unwrap();
        let registry = PluginRegistry::new(temp_dir.path().to_path_buf());
        
        // List should be empty initially
        let plugins = registry.list_plugins().await;
        assert!(plugins.is_empty());
        
        // TODO: Add more tests once we have a way to create test plugins
    }
    
    #[test]
    fn test_example_plugin() {
        let mut plugin = ExamplePlugin::new();
        
        assert_eq!(plugin.name(), "example");
        assert_eq!(plugin.version(), "0.1.0");
        assert_eq!(plugin.supported_agent_types(), vec!["example", "test"]);
        
        // Should initialize successfully
        let config = PluginConfig {
            name: "example".to_string(),
            version: "0.1.0".to_string(),
            settings: HashMap::new(),
        };
        assert!(plugin.initialize(config).is_ok());
        
        // Cleanup should work
        assert!(plugin.cleanup().is_ok());
    }
}