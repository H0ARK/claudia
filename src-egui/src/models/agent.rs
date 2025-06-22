use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AgentId(pub Uuid);

impl AgentId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AgentRole {
    Orchestrator {
        subordinates: Vec<AgentId>,
        max_parallel_tasks: usize,
    },
    Architect {
        specialization: ArchitectSpecialization,
        design_patterns: Vec<String>,
    },
    Developer {
        language_expertise: Vec<ProgrammingLanguage>,
        framework_expertise: Vec<Framework>,
        years_experience: f32,
    },
    Reviewer {
        review_types: Vec<ReviewType>,
        severity_threshold: ReviewSeverity,
    },
    Tester {
        test_strategies: Vec<TestStrategy>,
        coverage_target: f32,
    },
    DocumentationWriter {
        doc_formats: Vec<DocFormat>,
        technical_level: TechnicalLevel,
    },
    DataAnalyst {
        analysis_tools: Vec<String>,
        visualization_skills: Vec<String>,
    },
    DevOps {
        platforms: Vec<Platform>,
        ci_cd_tools: Vec<String>,
    },
    Custom {
        name: String,
        capabilities: Vec<String>,
        custom_prompt: String,
    },
    Assistant,
    Analyst,
    Designer,
    DataEngineer,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ArchitectSpecialization {
    SystemDesign,
    CloudArchitecture,
    Microservices,
    EventDriven,
    DataArchitecture,
    SecurityArchitecture,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProgrammingLanguage {
    Rust,
    Python,
    JavaScript,
    TypeScript,
    Go,
    Java,
    CSharp,
    Cpp,
    Other(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Framework {
    React,
    Vue,
    Angular,
    NextJs,
    Express,
    FastAPI,
    Django,
    Spring,
    DotNet,
    Actix,
    Tokio,
    Other(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReviewType {
    Code,
    Architecture,
    Security,
    Performance,
    Documentation,
    TestCoverage,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum ReviewSeverity {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TestStrategy {
    Unit,
    Integration,
    EndToEnd,
    Performance,
    Security,
    Accessibility,
    Regression,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DocFormat {
    Markdown,
    AsciiDoc,
    ReStructuredText,
    HTML,
    PDF,
    OpenAPI,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum TechnicalLevel {
    Beginner,
    Intermediate,
    Advanced,
    Expert,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Platform {
    AWS,
    Azure,
    GCP,
    Kubernetes,
    Docker,
    OnPremise,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum AgentStatus {
    Idle,
    Working {
        task_id: Uuid,
        progress: f32,
    },
    Waiting {
        waiting_for: Uuid,
    },
    Error {
        retry_count: u32,
    },
    Completed,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ModelType {
    Claude4Opus,
    Claude4Sonnet,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Permissions {
    pub file_read: bool,
    pub file_write: bool,
    pub network_access: bool,
    pub system_commands: bool,
    pub max_tokens: Option<u32>,
    pub allowed_paths: Vec<String>,
    pub blocked_paths: Vec<String>,
}

impl Default for Permissions {
    fn default() -> Self {
        Self {
            file_read: true,
            file_write: false,
            network_access: false,
            system_commands: false,
            max_tokens: Some(100_000),
            allowed_paths: vec![],
            blocked_paths: vec![],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Agent {
    pub id: AgentId,
    pub name: String,
    pub role: AgentRole,
    pub model: ModelType,
    pub system_prompt: String,
    pub permissions: Permissions,
    pub status: AgentStatus,
    pub metrics: AgentMetrics,
    pub created_at: DateTime<Utc>,
    pub last_active: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMetrics {
    pub total_tasks: u32,
    pub successful_tasks: u32,
    pub failed_tasks: u32,
    pub total_tokens_used: u64,
    pub average_response_time_ms: f64,
    pub total_cost_usd: f64,
}

impl Default for AgentMetrics {
    fn default() -> Self {
        Self {
            total_tasks: 0,
            successful_tasks: 0,
            failed_tasks: 0,
            total_tokens_used: 0,
            average_response_time_ms: 0.0,
            total_cost_usd: 0.0,
        }
    }
}

impl Agent {
    pub fn new(name: String, role: AgentRole, model: ModelType) -> Self {
        let system_prompt = Self::generate_system_prompt(&role);
        
        Self {
            id: AgentId::new(),
            name,
            role,
            model,
            system_prompt,
            permissions: Permissions::default(),
            status: AgentStatus::Idle,
            metrics: AgentMetrics::default(),
            created_at: Utc::now(),
            last_active: Utc::now(),
        }
    }

    fn generate_system_prompt(role: &AgentRole) -> String {
        match role {
            AgentRole::Orchestrator { .. } => {
                "You are an orchestrator agent responsible for coordinating tasks between other agents. \
                 You break down complex problems, assign tasks to appropriate specialists, and ensure \
                 efficient collaboration.".to_string()
            }
            AgentRole::Architect { specialization, .. } => {
                format!("You are a software architect specializing in {:?}. You design robust, \
                        scalable systems and provide architectural guidance.", specialization)
            }
            AgentRole::Developer { .. } => {
                "You are an expert software developer. You write clean, efficient, and well-tested code \
                 following best practices and design patterns.".to_string()
            }
            AgentRole::Reviewer { .. } => {
                "You are a code reviewer. You provide constructive feedback on code quality, \
                 identify potential issues, and suggest improvements.".to_string()
            }
            AgentRole::Tester { .. } => {
                "You are a QA engineer. You design and implement comprehensive test strategies \
                 to ensure software quality and reliability.".to_string()
            }
            AgentRole::DocumentationWriter { .. } => {
                "You are a technical writer. You create clear, comprehensive documentation \
                 that helps users and developers understand and use software effectively.".to_string()
            }
            AgentRole::DataAnalyst { .. } => {
                "You are a data analyst. You analyze data, identify patterns, and provide \
                 actionable insights through clear visualizations and reports.".to_string()
            }
            AgentRole::DevOps { .. } => {
                "You are a DevOps engineer. You design and implement CI/CD pipelines, \
                 manage infrastructure, and ensure smooth deployments.".to_string()
            }
            AgentRole::Custom { custom_prompt, .. } => custom_prompt.clone(),
            AgentRole::Assistant => {
                "You are a helpful AI assistant. You provide clear, accurate, and helpful responses \
                 to a wide variety of tasks and questions.".to_string()
            }
            AgentRole::Analyst => {
                "You are a business analyst. You analyze requirements, identify stakeholder needs, \
                 and help define clear project specifications.".to_string()
            }
            AgentRole::Designer => {
                "You are a UI/UX designer. You create intuitive, user-friendly interfaces and \
                 design beautiful user experiences.".to_string()
            }
            AgentRole::DataEngineer => {
                "You are a data engineer. You design and implement data pipelines, ETL processes, \
                 and ensure data quality and availability.".to_string()
            }
        }
    }

    pub fn can_work_on_task(&self, task_requirements: &HashMap<String, String>) -> bool {
        // Check if agent's role and capabilities match task requirements
        match &self.role {
            AgentRole::Developer { language_expertise, .. } => {
                if let Some(required_lang) = task_requirements.get("language") {
                    language_expertise.iter().any(|lang| {
                        format!("{:?}", lang).to_lowercase().contains(&required_lang.to_lowercase())
                    })
                } else {
                    true
                }
            }
            AgentRole::Reviewer { review_types, .. } => {
                if let Some(required_type) = task_requirements.get("review_type") {
                    review_types.iter().any(|rt| {
                        format!("{:?}", rt).to_lowercase().contains(&required_type.to_lowercase())
                    })
                } else {
                    true
                }
            }
            _ => true, // Other roles can work on any task by default
        }
    }
}