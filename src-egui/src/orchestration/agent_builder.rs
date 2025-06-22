use crate::models::TauriAgent;
use crate::utils::BackendBridge;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSpecification {
    pub name: String,
    pub description: String,
    pub capabilities: Vec<String>,
    pub required_permissions: Vec<String>,
    pub example_tasks: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratedAgent {
    pub agent: TauriAgent,
    pub reasoning: String,
    pub suggested_workflows: Vec<String>,
}

pub struct AgentBuilder {
    backend_bridge: Arc<BackendBridge>,
}

impl AgentBuilder {
    pub fn new(backend_bridge: Arc<BackendBridge>) -> Self {
        Self { backend_bridge }
    }

    /// Generate a new agent based on a natural language description
    pub async fn build_agent_from_description(&self, description: &str) -> Result<GeneratedAgent> {
        // Parse the description to extract agent requirements
        let spec = self.analyze_agent_requirements(description).await?;
        
        // Generate the system prompt for the agent
        let system_prompt = self.generate_system_prompt(&spec).await?;
        
        // Create the agent configuration
        let agent = TauriAgent {
            id: None,
            name: spec.name.clone(),
            icon: self.select_icon_for_agent(&spec),
            system_prompt,
            default_task: Some(format!("Execute {} tasks", spec.name.to_lowercase())),
            model: self.select_model_for_agent(&spec),
            sandbox_enabled: self.should_enable_sandbox(&spec),
            enable_file_read: spec.required_permissions.contains(&"file_read".to_string()),
            enable_file_write: spec.required_permissions.contains(&"file_write".to_string()),
            enable_network: spec.required_permissions.contains(&"network".to_string()),
            enable_system_commands: spec.required_permissions.contains(&"system_commands".to_string()),
            custom_instructions: Some(self.generate_custom_instructions(&spec)),
            sandbox_profile_id: None,
            created_at: None,
            updated_at: None,
        };
        
        // Generate reasoning and workflow suggestions
        let reasoning = self.generate_reasoning(&spec, &agent).await?;
        let suggested_workflows = self.suggest_workflows(&spec).await?;
        
        Ok(GeneratedAgent {
            agent,
            reasoning,
            suggested_workflows,
        })
    }

    /// Analyze the description to extract agent requirements
    async fn analyze_agent_requirements(&self, description: &str) -> Result<AgentSpecification> {
        // This would normally call Claude to analyze the description
        // For now, we'll use a simple implementation
        
        let name = self.extract_agent_name(description);
        let capabilities = self.extract_capabilities(description);
        let permissions = self.infer_required_permissions(&capabilities);
        let example_tasks = self.generate_example_tasks(&capabilities);
        
        Ok(AgentSpecification {
            name,
            description: description.to_string(),
            capabilities,
            required_permissions: permissions,
            example_tasks,
        })
    }

    /// Generate a system prompt for the agent based on its specification
    async fn generate_system_prompt(&self, spec: &AgentSpecification) -> Result<String> {
        let prompt = format!(
            r#"You are {}, a specialized AI agent with the following capabilities:

## Core Capabilities:
{}

## Your Role:
{}

## Guidelines:
1. Focus on your specialized capabilities and provide expert assistance in these areas
2. Be proactive in identifying opportunities to apply your skills
3. Maintain high quality standards in all outputs
4. Collaborate effectively with other agents when needed
5. Always validate your work before presenting results

## Example Tasks You Can Handle:
{}

## Important Notes:
- You have access to the following permissions: {}
- Work within these constraints to achieve the best results
- Ask for clarification when requirements are ambiguous
- Provide detailed feedback on task progress

Remember: You are an expert in your domain. Users rely on your specialized knowledge and capabilities."#,
            spec.name,
            spec.capabilities.iter().map(|c| format!("- {}", c)).collect::<Vec<_>>().join("\n"),
            spec.description,
            spec.example_tasks.iter().map(|t| format!("- {}", t)).collect::<Vec<_>>().join("\n"),
            spec.required_permissions.join(", ")
        );
        
        Ok(prompt)
    }

    /// Extract agent name from description
    fn extract_agent_name(&self, description: &str) -> String {
        // Simple heuristic: look for patterns like "agent for X", "X specialist", etc.
        let lower = description.to_lowercase();
        
        if let Some(pos) = lower.find("agent for ") {
            let start = pos + 10;
            let rest = &description[start..];
            if let Some(end) = rest.find([' ', ',', '.'].as_ref()) {
                return self.capitalize(&rest[..end]);
            }
        }
        
        if let Some(pos) = lower.find(" specialist") {
            if pos > 0 {
                let before = &description[..pos];
                if let Some(start) = before.rfind(' ') {
                    return self.capitalize(&before[start + 1..]);
                }
                return self.capitalize(before);
            }
        }
        
        // Default name
        "Custom Agent".to_string()
    }

    /// Extract capabilities from description
    fn extract_capabilities(&self, description: &str) -> Vec<String> {
        let mut capabilities = Vec::new();
        let lower = description.to_lowercase();
        
        // Check for common capability keywords
        let capability_keywords = vec![
            ("code", "Code generation and analysis"),
            ("test", "Testing and quality assurance"),
            ("review", "Code review and feedback"),
            ("design", "System design and architecture"),
            ("data", "Data processing and analysis"),
            ("api", "API development and integration"),
            ("frontend", "Frontend development"),
            ("backend", "Backend development"),
            ("database", "Database design and optimization"),
            ("security", "Security analysis and implementation"),
            ("documentation", "Documentation writing"),
            ("research", "Research and analysis"),
            ("optimize", "Performance optimization"),
            ("debug", "Debugging and troubleshooting"),
            ("deploy", "Deployment and DevOps"),
        ];
        
        for (keyword, capability) in capability_keywords {
            if lower.contains(keyword) {
                capabilities.push(capability.to_string());
            }
        }
        
        if capabilities.is_empty() {
            capabilities.push("General task execution".to_string());
        }
        
        capabilities
    }

    /// Infer required permissions based on capabilities
    fn infer_required_permissions(&self, capabilities: &[String]) -> Vec<String> {
        let mut permissions = Vec::new();
        
        for capability in capabilities {
            let cap_lower = capability.to_lowercase();
            
            if cap_lower.contains("file") || cap_lower.contains("code") || cap_lower.contains("documentation") {
                permissions.push("file_read".to_string());
            }
            
            if cap_lower.contains("generate") || cap_lower.contains("write") || cap_lower.contains("create") {
                permissions.push("file_write".to_string());
            }
            
            if cap_lower.contains("api") || cap_lower.contains("integration") || cap_lower.contains("deploy") {
                permissions.push("network".to_string());
            }
            
            if cap_lower.contains("build") || cap_lower.contains("test") || cap_lower.contains("deploy") {
                permissions.push("system_commands".to_string());
            }
        }
        
        // Deduplicate
        permissions.sort();
        permissions.dedup();
        
        permissions
    }

    /// Generate example tasks based on capabilities
    fn generate_example_tasks(&self, capabilities: &[String]) -> Vec<String> {
        let mut tasks = Vec::new();
        
        for capability in capabilities {
            let cap_lower = capability.to_lowercase();
            
            if cap_lower.contains("code generation") {
                tasks.push("Generate boilerplate code for a REST API".to_string());
                tasks.push("Create unit tests for existing functions".to_string());
            }
            
            if cap_lower.contains("review") {
                tasks.push("Review pull requests for code quality".to_string());
                tasks.push("Identify potential security vulnerabilities".to_string());
            }
            
            if cap_lower.contains("data") {
                tasks.push("Process and analyze CSV data files".to_string());
                tasks.push("Generate data visualization reports".to_string());
            }
            
            if cap_lower.contains("documentation") {
                tasks.push("Generate API documentation from code".to_string());
                tasks.push("Create user guides and tutorials".to_string());
            }
        }
        
        if tasks.is_empty() {
            tasks.push("Execute specialized tasks in the agent's domain".to_string());
        }
        
        tasks
    }

    /// Select appropriate icon for the agent
    fn select_icon_for_agent(&self, spec: &AgentSpecification) -> String {
        let name_lower = spec.name.to_lowercase();
        let desc_lower = spec.description.to_lowercase();
        
        // Icon selection based on keywords
        if name_lower.contains("test") || desc_lower.contains("test") {
            "🧪".to_string()
        } else if name_lower.contains("security") || desc_lower.contains("security") {
            "🔒".to_string()
        } else if name_lower.contains("data") || desc_lower.contains("data") {
            "📊".to_string()
        } else if name_lower.contains("api") || desc_lower.contains("api") {
            "🔌".to_string()
        } else if name_lower.contains("design") || desc_lower.contains("design") {
            "🎨".to_string()
        } else if name_lower.contains("doc") || desc_lower.contains("documentation") {
            "📝".to_string()
        } else if name_lower.contains("deploy") || desc_lower.contains("devops") {
            "🚀".to_string()
        } else if name_lower.contains("review") || desc_lower.contains("review") {
            "👁️".to_string()
        } else if name_lower.contains("research") || desc_lower.contains("research") {
            "🔍".to_string()
        } else {
            "🤖".to_string()
        }
    }

    /// Select appropriate model based on agent requirements
    fn select_model_for_agent(&self, spec: &AgentSpecification) -> String {
        // Use Opus for complex tasks requiring deep reasoning
        let complex_keywords = vec!["architect", "design", "research", "analyze", "optimize"];
        for keyword in complex_keywords {
            if spec.description.to_lowercase().contains(keyword) {
                return "claude-3-opus-20240229".to_string();
            }
        }
        
        // Use Haiku for simple, fast tasks
        let simple_keywords = vec!["format", "simple", "quick", "basic"];
        for keyword in simple_keywords {
            if spec.description.to_lowercase().contains(keyword) {
                return "claude-3-haiku-20240307".to_string();
            }
        }
        
        // Default to Sonnet for balanced performance
        "claude-3-sonnet-20240229".to_string()
    }

    /// Determine if sandbox should be enabled
    fn should_enable_sandbox(&self, spec: &AgentSpecification) -> bool {
        // Enable sandbox for agents that need system commands
        spec.required_permissions.contains(&"system_commands".to_string())
    }

    /// Generate custom instructions for the agent
    fn generate_custom_instructions(&self, spec: &AgentSpecification) -> String {
        format!(
            "This agent specializes in: {}. Key capabilities include: {}.",
            spec.description,
            spec.capabilities.join(", ")
        )
    }

    /// Generate reasoning about the agent configuration
    async fn generate_reasoning(&self, spec: &AgentSpecification, agent: &TauriAgent) -> Result<String> {
        let reasoning = format!(
            r#"## Agent Configuration Reasoning

**Name**: {} was chosen based on the primary function described.

**Model Selection**: {} was selected because:
- The task complexity matches this model's capabilities
- It provides the right balance of performance and cost

**Permissions**:
- File Read: {} - {}
- File Write: {} - {}
- Network: {} - {}
- System Commands: {} - {}

**Icon**: {} was chosen to visually represent the agent's primary function.

**Key Design Decisions**:
1. The system prompt emphasizes the agent's specialized capabilities
2. Permissions are limited to what's necessary for the described tasks
3. The model choice optimizes for the expected workload type"#,
            agent.name,
            agent.model,
            agent.enable_file_read,
            if agent.enable_file_read { "Required for reading and analyzing files" } else { "Not needed for the described tasks" },
            agent.enable_file_write,
            if agent.enable_file_write { "Required for generating and modifying files" } else { "Not needed for the described tasks" },
            agent.enable_network,
            if agent.enable_network { "Required for API calls and external integrations" } else { "Not needed for the described tasks" },
            agent.enable_system_commands,
            if agent.enable_system_commands { "Required for running build/test commands" } else { "Not needed for the described tasks" },
            agent.icon
        );
        
        Ok(reasoning)
    }

    /// Suggest workflows that could use this agent
    async fn suggest_workflows(&self, spec: &AgentSpecification) -> Result<Vec<String>> {
        let mut workflows = Vec::new();
        
        // Suggest workflows based on capabilities
        for capability in &spec.capabilities {
            let cap_lower = capability.to_lowercase();
            
            if cap_lower.contains("test") {
                workflows.push("Automated Testing Pipeline".to_string());
                workflows.push("Pull Request Validation Workflow".to_string());
            }
            
            if cap_lower.contains("code") && cap_lower.contains("generation") {
                workflows.push("Code Scaffolding Workflow".to_string());
                workflows.push("API Endpoint Generation Pipeline".to_string());
            }
            
            if cap_lower.contains("review") {
                workflows.push("Code Review Automation".to_string());
                workflows.push("Security Audit Workflow".to_string());
            }
            
            if cap_lower.contains("deploy") {
                workflows.push("CI/CD Pipeline".to_string());
                workflows.push("Production Deployment Workflow".to_string());
            }
            
            if cap_lower.contains("data") {
                workflows.push("Data Processing Pipeline".to_string());
                workflows.push("Report Generation Workflow".to_string());
            }
        }
        
        // Deduplicate
        workflows.sort();
        workflows.dedup();
        
        if workflows.is_empty() {
            workflows.push(format!("{} Workflow", spec.name));
        }
        
        Ok(workflows)
    }

    /// Helper function to capitalize string
    fn capitalize(&self, s: &str) -> String {
        let mut chars = s.chars();
        match chars.next() {
            None => String::new(),
            Some(first) => first.to_uppercase().chain(chars).collect(),
        }
    }

    /// Save the generated agent to the database
    pub async fn save_agent(&self, agent: &TauriAgent) -> Result<i64> {
        self.backend_bridge.create_agent(agent)
    }
}