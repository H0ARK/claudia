# Egui-based Multi-Role Orchestrator Design

## Overview

This document outlines the architectural design for replacing the React/TypeScript frontend with an egui-based Rust frontend, focusing on creating a clean multi-role orchestrator for Claude agents.

## Architecture

### 1. Project Structure

```
claudia/
├── src-tauri/           # Existing backend (minimal changes)
├── src-egui/            # New egui frontend
│   ├── main.rs          # Entry point & app initialization
│   ├── app.rs           # Main application state
│   ├── ui/              # UI components
│   │   ├── mod.rs
│   │   ├── orchestrator.rs    # Main orchestrator view
│   │   ├── agent_manager.rs   # Agent creation/management
│   │   ├── workflow_editor.rs # Visual workflow builder
│   │   ├── execution_view.rs  # Real-time execution monitoring
│   │   └── metrics_panel.rs   # Performance & usage metrics
│   ├── orchestration/   # Core orchestration logic
│   │   ├── mod.rs
│   │   ├── engine.rs    # Workflow execution engine
│   │   ├── roles.rs     # Agent role definitions
│   │   ├── communication.rs # Inter-agent messaging
│   │   └── scheduler.rs # Task scheduling & coordination
│   ├── models/          # Data models
│   │   ├── mod.rs
│   │   ├── agent.rs
│   │   ├── workflow.rs
│   │   └── message.rs
│   └── utils/           # Utilities
│       ├── mod.rs
│       └── ipc.rs       # Tauri IPC communication
├── Cargo.toml           # Updated workspace configuration
└── README.md

```

### 2. Core Components

#### 2.1 Multi-Role Agent System

```rust
// Agent roles hierarchy
pub enum AgentRole {
    Orchestrator {
        subordinates: Vec<AgentId>,
        workflow: WorkflowDefinition,
    },
    Architect {
        specialization: ArchitectSpecialization,
    },
    Developer {
        language_expertise: Vec<Language>,
        framework_expertise: Vec<Framework>,
    },
    Reviewer {
        review_type: ReviewType,
    },
    Tester {
        test_strategy: TestStrategy,
    },
    DocumentationWriter {
        doc_format: DocFormat,
    },
    Custom {
        name: String,
        capabilities: Vec<Capability>,
    },
}

pub struct AgentDefinition {
    pub id: AgentId,
    pub name: String,
    pub role: AgentRole,
    pub model: ModelType,
    pub system_prompt: String,
    pub permissions: Permissions,
    pub communication_channels: Vec<ChannelId>,
}
```

#### 2.2 Orchestration Engine

```rust
pub struct OrchestrationEngine {
    agents: HashMap<AgentId, RunningAgent>,
    workflows: HashMap<WorkflowId, WorkflowState>,
    message_bus: MessageBus,
    scheduler: TaskScheduler,
}

impl OrchestrationEngine {
    pub async fn execute_workflow(&mut self, workflow: WorkflowDefinition) -> Result<WorkflowResult>;
    pub async fn dispatch_task(&mut self, task: Task, agent_id: AgentId) -> Result<TaskResult>;
    pub async fn coordinate_agents(&mut self, coordination_plan: CoordinationPlan) -> Result<()>;
}
```

#### 2.3 Inter-Agent Communication

```rust
pub enum MessageType {
    TaskAssignment { task: Task, deadline: Option<DateTime> },
    StatusUpdate { progress: f32, details: String },
    ResultDelivery { task_id: TaskId, result: TaskResult },
    Collaboration { request: CollaborationRequest },
    Review { artifact: Artifact, review_type: ReviewType },
    Approval { request_id: RequestId, approved: bool, feedback: Option<String> },
}

pub struct Message {
    pub id: MessageId,
    pub from: AgentId,
    pub to: AgentTarget,
    pub message_type: MessageType,
    pub priority: Priority,
    pub timestamp: DateTime,
}

pub enum AgentTarget {
    Specific(AgentId),
    Role(AgentRole),
    Broadcast,
    Group(Vec<AgentId>),
}
```

### 3. UI Design with egui

#### 3.1 Main Orchestrator View

```rust
pub struct OrchestratorView {
    workflow_graph: WorkflowGraph,
    agent_status_panel: AgentStatusPanel,
    execution_timeline: ExecutionTimeline,
    message_log: MessageLog,
}

impl OrchestratorView {
    pub fn ui(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            // Main workflow visualization
            self.workflow_graph.show(ui);
        });

        egui::SidePanel::left("agents").show(ctx, |ui| {
            // Agent status and controls
            self.agent_status_panel.show(ui);
        });

        egui::TopBottomPanel::bottom("timeline").show(ctx, |ui| {
            // Execution timeline
            self.execution_timeline.show(ui);
        });
    }
}
```

#### 3.2 Workflow Editor

```rust
pub struct WorkflowEditor {
    nodes: Vec<WorkflowNode>,
    connections: Vec<Connection>,
    selected_node: Option<NodeId>,
}

impl WorkflowEditor {
    pub fn ui(&mut self, ctx: &egui::Context) {
        // Visual node-based workflow editor
        // Drag & drop agent roles
        // Connect nodes to define task flow
        // Set conditions and branching logic
    }
}
```

### 4. Key Features

#### 4.1 Workflow Templates

Pre-built workflow templates for common scenarios:
- **Full Stack Development**: Architect → Backend Dev → Frontend Dev → Reviewer → Tester
- **Bug Fix Pipeline**: Analyzer → Developer → Tester → Reviewer
- **Feature Implementation**: Product Manager → Architect → Developers → Documentation
- **Code Review**: Developer → Reviewer → Tester → Approver

#### 4.2 Real-time Monitoring

- Live agent status indicators
- Message flow visualization
- Token usage per agent
- Execution timeline with playback
- Performance metrics dashboard

#### 4.3 Advanced Orchestration

- Conditional branching based on agent outputs
- Parallel task execution
- Agent pooling for load balancing
- Automatic retry with fallback agents
- Priority-based task scheduling

### 5. Integration Strategy

#### 5.1 Backend Integration

The existing Tauri backend will be minimally modified:
- Extend agent execution API to support orchestration commands
- Add new IPC channels for inter-agent communication
- Enhance process registry for multi-agent tracking
- Update database schema for workflow persistence

#### 5.2 Migration Path

1. **Phase 1**: Create egui app structure alongside React
2. **Phase 2**: Implement core orchestration engine
3. **Phase 3**: Build essential UI components
4. **Phase 4**: Connect to existing backend APIs
5. **Phase 5**: Add advanced features
6. **Phase 6**: Remove React frontend

### 6. Benefits of egui Approach

- **Performance**: Native Rust performance, no JavaScript overhead
- **Type Safety**: Compile-time guarantees for complex orchestration logic
- **Resource Efficiency**: Lower memory footprint, faster rendering
- **Unified Codebase**: Frontend and orchestration logic in same language
- **Better Integration**: Direct access to system resources without IPC overhead
- **Real-time Visualization**: Efficient rendering of complex workflow graphs

### 7. Example Workflow Definition

```rust
let web_app_workflow = WorkflowDefinition {
    name: "Full Stack Web App Development".to_string(),
    stages: vec![
        Stage {
            name: "Architecture Design",
            agent_role: AgentRole::Architect,
            inputs: vec!["requirements.md"],
            outputs: vec!["architecture.md", "api_spec.yaml"],
        },
        Stage {
            name: "Parallel Development",
            parallel: true,
            substages: vec![
                SubStage {
                    name: "Backend Implementation",
                    agent_role: AgentRole::Developer { 
                        language_expertise: vec![Language::Rust],
                        framework_expertise: vec![Framework::Actix],
                    },
                    inputs: vec!["architecture.md", "api_spec.yaml"],
                    outputs: vec!["backend/"],
                },
                SubStage {
                    name: "Frontend Implementation",
                    agent_role: AgentRole::Developer {
                        language_expertise: vec![Language::TypeScript],
                        framework_expertise: vec![Framework::React],
                    },
                    inputs: vec!["architecture.md", "api_spec.yaml"],
                    outputs: vec!["frontend/"],
                },
            ],
        },
        Stage {
            name: "Integration Testing",
            agent_role: AgentRole::Tester,
            inputs: vec!["backend/", "frontend/"],
            outputs: vec!["test_report.md"],
        },
        Stage {
            name: "Code Review",
            agent_role: AgentRole::Reviewer,
            inputs: vec!["backend/", "frontend/", "test_report.md"],
            outputs: vec!["review_report.md", "approved: bool"],
        },
    ],
    error_handling: ErrorStrategy::RetryWithDifferentAgent,
    max_retries: 2,
    timeout: Duration::from_secs(3600),
};
```

This design provides a solid foundation for building a powerful multi-role orchestrator system with egui that can coordinate complex Claude agent workflows efficiently.