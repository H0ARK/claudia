use crate::models::agent::{AgentId, AgentRole};
use chrono::{DateTime, Duration, Utc};
use petgraph::graph::{DiGraph, NodeIndex};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WorkflowId(pub Uuid);

impl WorkflowId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workflow {
    pub id: WorkflowId,
    pub name: String,
    pub description: String,
    pub graph: WorkflowGraph,
    pub variables: HashMap<String, String>,
    pub error_strategy: ErrorStrategy,
    pub max_retries: u32,
    pub timeout: Option<Duration>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowGraph {
    #[serde(skip)]
    pub graph: DiGraph<WorkflowNode, WorkflowEdge>,
    pub nodes: Vec<WorkflowNode>,
    pub edges: Vec<(usize, usize, WorkflowEdge)>,
}

impl WorkflowGraph {
    pub fn new() -> Self {
        Self {
            graph: DiGraph::new(),
            nodes: Vec::new(),
            edges: Vec::new(),
        }
    }

    pub fn add_node(&mut self, node: WorkflowNode) -> NodeIndex {
        let idx = self.graph.add_node(node.clone());
        self.nodes.push(node);
        idx
    }

    pub fn add_edge(&mut self, from: NodeIndex, to: NodeIndex, edge: WorkflowEdge) {
        self.graph.add_edge(from, to, edge.clone());
        self.edges.push((from.index(), to.index(), edge));
    }

    pub fn rebuild_graph(&mut self) {
        self.graph = DiGraph::new();
        let mut node_map = HashMap::new();

        // Rebuild nodes
        for (i, node) in self.nodes.iter().enumerate() {
            let idx = self.graph.add_node(node.clone());
            node_map.insert(i, idx);
        }

        // Rebuild edges
        for (from, to, edge) in &self.edges {
            if let (Some(&from_idx), Some(&to_idx)) = (node_map.get(from), node_map.get(to)) {
                self.graph.add_edge(from_idx, to_idx, edge.clone());
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowNode {
    pub id: Uuid,
    pub name: String,
    pub node_type: WorkflowNodeType,
    pub position: (f32, f32), // For visual representation
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WorkflowNodeType {
    Start,
    End,
    Task(WorkflowTask),
    Parallel(Vec<WorkflowTask>),
    Conditional {
        condition: String,
        true_branch: Box<WorkflowNode>,
        false_branch: Option<Box<WorkflowNode>>,
    },
    Loop {
        condition: String,
        body: Box<WorkflowNode>,
        max_iterations: Option<u32>,
    },
    Wait {
        duration: Duration,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowTask {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub agent_role: AgentRole,
    pub assigned_agent: Option<AgentId>,
    pub inputs: Vec<TaskInput>,
    pub outputs: Vec<TaskOutput>,
    pub timeout: Option<Duration>,
    pub retry_policy: RetryPolicy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskInput {
    pub name: String,
    pub input_type: DataType,
    pub source: DataSource,
    pub required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskOutput {
    pub name: String,
    pub output_type: DataType,
    pub destination: DataDestination,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DataType {
    Text,
    Code { language: String },
    File { mime_type: String },
    Json,
    Binary,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DataSource {
    UserInput,
    PreviousTask { task_id: Uuid, output_name: String },
    Variable { name: String },
    Literal { value: String },
    File { path: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DataDestination {
    Variable { name: String },
    File { path: String },
    NextTask,
    User,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowEdge {
    pub condition: Option<String>,
    pub priority: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ErrorStrategy {
    Fail,
    Retry,
    RetryWithDifferentAgent,
    SkipAndContinue,
    Fallback { fallback_workflow: WorkflowId },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryPolicy {
    pub max_attempts: u32,
    pub backoff_strategy: BackoffStrategy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BackoffStrategy {
    Fixed { delay_ms: u64 },
    Linear { initial_delay_ms: u64, increment_ms: u64 },
    Exponential { initial_delay_ms: u64, multiplier: f64 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowStage {
    pub name: String,
    pub tasks: Vec<WorkflowTask>,
    pub parallel: bool,
    pub dependencies: Vec<String>,
}

impl Workflow {
    pub fn new(name: String, description: String) -> Self {
        let mut graph = WorkflowGraph::new();
        
        // Add start and end nodes
        graph.add_node(WorkflowNode {
            id: Uuid::new_v4(),
            name: "Start".to_string(),
            node_type: WorkflowNodeType::Start,
            position: (100.0, 300.0),
        });
        
        graph.add_node(WorkflowNode {
            id: Uuid::new_v4(),
            name: "End".to_string(),
            node_type: WorkflowNodeType::End,
            position: (700.0, 300.0),
        });

        Self {
            id: WorkflowId::new(),
            name,
            description,
            graph,
            variables: HashMap::new(),
            error_strategy: ErrorStrategy::Retry,
            max_retries: 3,
            timeout: Some(Duration::hours(1)),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    pub fn add_task(&mut self, task: WorkflowTask, after: Option<NodeIndex>) -> NodeIndex {
        let node = WorkflowNode {
            id: task.id,
            name: task.name.clone(),
            node_type: WorkflowNodeType::Task(task),
            position: (400.0, 300.0), // Default position
        };

        let idx = self.graph.add_node(node);

        if let Some(prev_idx) = after {
            self.graph.add_edge(prev_idx, idx, WorkflowEdge {
                condition: None,
                priority: 0,
            });
        }

        self.updated_at = Utc::now();
        idx
    }

    pub fn validate(&self) -> Result<(), String> {
        // Check for cycles
        if petgraph::algo::is_cyclic_directed(&self.graph.graph) {
            return Err("Workflow contains cycles".to_string());
        }

        // Check all paths lead to end
        // TODO: Implement path validation

        // Check all required inputs are satisfied
        // TODO: Implement input validation

        Ok(())
    }
}