use crate::orchestration::{PluginRegistry, PluginMetadata};
use std::path::PathBuf;
use std::sync::Arc;
use tauri::{AppHandle, Manager, State};
use tokio::sync::RwLock;

/// State wrapper for the plugin registry
pub struct PluginState(pub Arc<RwLock<Option<PluginRegistry>>>);

impl Default for PluginState {
    fn default() -> Self {
        Self(Arc::new(RwLock::new(None)))
    }
}

/// Initialize the plugin registry
#[tauri::command]
pub async fn init_plugin_registry(
    app: AppHandle,
    state: State<'_, PluginState>,
) -> Result<String, String> {
    let mut registry_opt = state.0.write().await;
    
    if registry_opt.is_some() {
        return Ok("Plugin registry already initialized".to_string());
    }
    
    // Get plugin directory
    let app_data_dir = app.path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;
    let plugin_dir = app_data_dir.join("plugins");
    
    // Create plugin registry
    let registry = PluginRegistry::new(plugin_dir);
    
    // Scan and load existing plugins
    match registry.scan_and_load_plugins().await {
        Ok(loaded) => {
            let count = loaded.len();
            *registry_opt = Some(registry);
            Ok(format!("Plugin registry initialized. Loaded {} plugins", count))
        }
        Err(e) => {
            *registry_opt = Some(registry);
            Ok(format!("Plugin registry initialized with warning: {}", e))
        }
    }
}

/// Load a plugin from a file path
#[tauri::command]
pub async fn load_plugin(
    state: State<'_, PluginState>,
    path: String,
) -> Result<String, String> {
    let registry_opt = state.0.read().await;
    let registry = registry_opt.as_ref()
        .ok_or_else(|| "Plugin registry not initialized".to_string())?;
    
    let plugin_path = PathBuf::from(path);
    registry.load_plugin(&plugin_path).await
        .map_err(|e| format!("Failed to load plugin: {}", e))
}

/// Unload a plugin
#[tauri::command]
pub async fn unload_plugin(
    state: State<'_, PluginState>,
    name: String,
) -> Result<(), String> {
    let registry_opt = state.0.read().await;
    let registry = registry_opt.as_ref()
        .ok_or_else(|| "Plugin registry not initialized".to_string())?;
    
    registry.unload_plugin(&name).await
        .map_err(|e| format!("Failed to unload plugin: {}", e))
}

/// List all loaded plugins
#[tauri::command]
pub async fn list_plugins(
    state: State<'_, PluginState>,
) -> Result<Vec<PluginMetadata>, String> {
    let registry_opt = state.0.read().await;
    let registry = registry_opt.as_ref()
        .ok_or_else(|| "Plugin registry not initialized".to_string())?;
    
    Ok(registry.list_plugins().await)
}

/// Find a plugin for a specific agent type
#[tauri::command]
pub async fn find_plugin_for_agent_type(
    state: State<'_, PluginState>,
    agent_type: String,
) -> Result<Option<String>, String> {
    let registry_opt = state.0.read().await;
    let registry = registry_opt.as_ref()
        .ok_or_else(|| "Plugin registry not initialized".to_string())?;
    
    Ok(registry.find_plugin_for_agent_type(&agent_type).await)
}

/// Execute a task using a plugin
#[tauri::command]
pub async fn execute_plugin_task(
    state: State<'_, PluginState>,
    plugin_name: String,
    task_id: i64,
    goal: String,
    context: serde_json::Value,
) -> Result<(), String> {
    let registry_opt = state.0.read().await;
    let registry = registry_opt.as_ref()
        .ok_or_else(|| "Plugin registry not initialized".to_string())?;
    
    // Convert JSON context to TaskContext
    let task_context: crate::orchestration::TaskContext = serde_json::from_value(context)
        .map_err(|e| format!("Invalid task context: {}", e))?;
    
    // Create a channel for receiving messages from the plugin
    let (tx, mut rx) = tokio::sync::mpsc::channel(100);
    
    // Create callback
    let callback = Box::new(crate::orchestration::KernelCallback::new(tx));
    
    // Execute task in background
    let registry_clone = registry_opt.clone();
    let plugin_name_clone = plugin_name.clone();
    tokio::spawn(async move {
        if let Some(registry) = registry_clone.as_ref() {
            let _ = registry.execute_task(
                &plugin_name_clone,
                task_id,
                goal,
                task_context,
                callback,
            ).await;
        }
    });
    
    // Handle messages from plugin
    tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            // TODO: Forward messages to the orchestration kernel
            log::debug!("Plugin message: {:?}", msg);
        }
    });
    
    Ok(())
}

/// Scan plugin directory and reload plugins
#[tauri::command]
pub async fn scan_plugins(
    state: State<'_, PluginState>,
) -> Result<Vec<String>, String> {
    let registry_opt = state.0.read().await;
    let registry = registry_opt.as_ref()
        .ok_or_else(|| "Plugin registry not initialized".to_string())?;
    
    registry.scan_and_load_plugins().await
        .map_err(|e| format!("Failed to scan plugins: {}", e))
}