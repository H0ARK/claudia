// Example standalone plugin that can be compiled as a dynamic library
// Compile with: rustc --crate-type cdylib example_plugin.rs

use std::collections::HashMap;
use std::path::PathBuf;

// Plugin trait definition (must match the one in mod.rs)
pub trait Plugin: Send + Sync {
    fn name(&self) -> &str;
    fn version(&self) -> &str;
    fn description(&self) -> &str;
    fn supported_agent_types(&self) -> Vec<String>;
    fn initialize(&mut self, config: PluginConfig) -> Result<(), Box<dyn std::error::Error>>;
    fn execute_task(
        &self,
        task_id: i64,
        goal: String,
        context: TaskContext,
        callback: Box<dyn PluginCallback>,
    ) -> Result<(), Box<dyn std::error::Error>>;
    fn cleanup(&mut self) -> Result<(), Box<dyn std::error::Error>>;
}

// Supporting types (simplified versions)
#[derive(Debug, Clone)]
pub struct PluginConfig {
    pub name: String,
    pub version: String,
    pub settings: HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct TaskContext {
    pub orchestration_id: i64,
    pub parent_tasks: Vec<ParentTaskInfo>,
    pub mounted_paths: HashMap<String, PathBuf>,
    pub environment: HashMap<String, String>,
    pub constraints: Vec<String>,
    pub available_tools: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ParentTaskInfo {
    pub task_id: i64,
    pub name: String,
    pub summary: String,
}

pub trait PluginCallback: Send + Sync {
    fn on_progress(&self, task_id: i64, progress: f32, message: String) -> Result<(), Box<dyn std::error::Error>>;
    fn on_log(&self, task_id: i64, level: &str, message: String) -> Result<(), Box<dyn std::error::Error>>;
    fn on_complete(&self, task_id: i64, summary: String) -> Result<(), Box<dyn std::error::Error>>;
    fn on_error(&self, task_id: i64, error: String, recoverable: bool) -> Result<(), Box<dyn std::error::Error>>;
}

// File analyzer plugin implementation
pub struct FileAnalyzerPlugin {
    name: String,
    version: String,
    initialized: bool,
    config: Option<PluginConfig>,
}

impl FileAnalyzerPlugin {
    pub fn new() -> Self {
        Self {
            name: "file_analyzer".to_string(),
            version: "1.0.0".to_string(),
            initialized: false,
            config: None,
        }
    }
    
    fn analyze_file(&self, path: &PathBuf) -> Result<String, Box<dyn std::error::Error>> {
        use std::fs;
        
        let metadata = fs::metadata(path)?;
        let content = if metadata.len() < 1024 * 1024 { // Less than 1MB
            fs::read_to_string(path).unwrap_or_else(|_| "[Binary content]".to_string())
        } else {
            "[Large file - content not loaded]".to_string()
        };
        
        let analysis = format!(
            "File: {:?}\nSize: {} bytes\nModified: {:?}\nContent preview:\n{}",
            path.file_name().unwrap_or_default(),
            metadata.len(),
            metadata.modified()?,
            &content.chars().take(500).collect::<String>()
        );
        
        Ok(analysis)
    }
}

impl Plugin for FileAnalyzerPlugin {
    fn name(&self) -> &str {
        &self.name
    }
    
    fn version(&self) -> &str {
        &self.version
    }
    
    fn description(&self) -> &str {
        "Analyzes files and provides detailed information about their content"
    }
    
    fn supported_agent_types(&self) -> Vec<String> {
        vec!["file_analyzer".to_string(), "analyzer".to_string()]
    }
    
    fn initialize(&mut self, config: PluginConfig) -> Result<(), Box<dyn std::error::Error>> {
        self.config = Some(config);
        self.initialized = true;
        Ok(())
    }
    
    fn execute_task(
        &self,
        task_id: i64,
        goal: String,
        context: TaskContext,
        callback: Box<dyn PluginCallback>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if !self.initialized {
            return Err("Plugin not initialized".into());
        }
        
        // Parse the goal to extract file paths
        callback.on_log(task_id, "info", format!("Starting file analysis task: {}", goal))?;
        callback.on_progress(task_id, 0.0, "Parsing task goal...".to_string())?;
        
        // Look for mounted paths that might contain files to analyze
        let mut files_analyzed = 0;
        let total_files = context.mounted_paths.len();
        
        for (name, path) in &context.mounted_paths {
            callback.on_progress(
                task_id,
                (files_analyzed as f32 / total_files as f32) * 100.0,
                format!("Analyzing {}", name),
            )?;
            
            match self.analyze_file(path) {
                Ok(analysis) => {
                    callback.on_log(task_id, "info", format!("Analysis of {}:\n{}", name, analysis))?;
                    files_analyzed += 1;
                }
                Err(e) => {
                    callback.on_log(task_id, "warning", format!("Failed to analyze {}: {}", name, e))?;
                }
            }
        }
        
        callback.on_progress(task_id, 100.0, "Analysis complete".to_string())?;
        callback.on_complete(
            task_id,
            format!("Successfully analyzed {} files", files_analyzed),
        )?;
        
        Ok(())
    }
    
    fn cleanup(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.initialized = false;
        self.config = None;
        Ok(())
    }
}

// Export function for creating the plugin
#[no_mangle]
pub extern "C" fn create_plugin() -> Box<dyn Plugin> {
    Box::new(FileAnalyzerPlugin::new())
}

// Code generation plugin implementation
pub struct CodeGeneratorPlugin {
    name: String,
    version: String,
    initialized: bool,
    templates: HashMap<String, String>,
}

impl CodeGeneratorPlugin {
    pub fn new() -> Self {
        let mut templates = HashMap::new();
        
        // Add some basic templates
        templates.insert("rust_struct".to_string(), r#"
#[derive(Debug, Clone)]
pub struct {{name}} {
    {{fields}}
}

impl {{name}} {
    pub fn new({{constructor_params}}) -> Self {
        Self {
            {{field_assignments}}
        }
    }
}
"#.to_string());
        
        templates.insert("typescript_interface".to_string(), r#"
export interface {{name}} {
    {{fields}}
}

export class {{name}}Impl implements {{name}} {
    {{field_declarations}}
    
    constructor({{constructor_params}}) {
        {{field_assignments}}
    }
}
"#.to_string());
        
        Self {
            name: "code_generator".to_string(),
            version: "1.0.0".to_string(),
            initialized: false,
            templates,
        }
    }
    
    fn generate_code(&self, template: &str, params: HashMap<String, String>) -> String {
        let mut result = template.to_string();
        
        for (key, value) in params {
            result = result.replace(&format!("{{{{{}}}}}", key), &value);
        }
        
        result
    }
}

impl Plugin for CodeGeneratorPlugin {
    fn name(&self) -> &str {
        &self.name
    }
    
    fn version(&self) -> &str {
        &self.version
    }
    
    fn description(&self) -> &str {
        "Generates code from templates based on specifications"
    }
    
    fn supported_agent_types(&self) -> Vec<String> {
        vec!["code_generator".to_string(), "generator".to_string()]
    }
    
    fn initialize(&mut self, _config: PluginConfig) -> Result<(), Box<dyn std::error::Error>> {
        self.initialized = true;
        Ok(())
    }
    
    fn execute_task(
        &self,
        task_id: i64,
        goal: String,
        _context: TaskContext,
        callback: Box<dyn PluginCallback>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if !self.initialized {
            return Err("Plugin not initialized".into());
        }
        
        callback.on_log(task_id, "info", format!("Starting code generation task: {}", goal))?;
        callback.on_progress(task_id, 0.0, "Parsing generation request...".to_string())?;
        
        // Simple example: generate a Rust struct
        // In a real implementation, this would parse the goal more intelligently
        let mut params = HashMap::new();
        params.insert("name".to_string(), "GeneratedStruct".to_string());
        params.insert("fields".to_string(), "pub id: u64,\n    pub name: String,".to_string());
        params.insert("constructor_params".to_string(), "id: u64, name: String".to_string());
        params.insert("field_assignments".to_string(), "id,\n            name,".to_string());
        
        callback.on_progress(task_id, 50.0, "Generating code...".to_string())?;
        
        if let Some(template) = self.templates.get("rust_struct") {
            let generated_code = self.generate_code(template, params);
            
            callback.on_log(task_id, "info", format!("Generated code:\n{}", generated_code))?;
            callback.on_progress(task_id, 100.0, "Code generation complete".to_string())?;
            callback.on_complete(
                task_id,
                "Successfully generated code based on template".to_string(),
            )?;
        } else {
            callback.on_error(task_id, "Template not found".to_string(), true)?;
        }
        
        Ok(())
    }
    
    fn cleanup(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.initialized = false;
        Ok(())
    }
}