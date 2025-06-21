# Claudia Orchestrator - Egui Frontend

This is the new egui-based frontend for Claudia, focusing on multi-role agent orchestration.

## Architecture Overview

The egui frontend is structured as follows:

```
src-egui/
├── src/
│   ├── main.rs              # Application entry point
│   ├── app.rs               # Main application state and navigation
│   ├── models/              # Data models
│   │   ├── agent.rs         # Agent definitions with roles and capabilities
│   │   ├── workflow.rs      # Workflow graphs and execution models
│   │   └── message.rs       # Inter-agent communication structures
│   ├── orchestration/       # Core orchestration logic
│   │   ├── engine.rs        # Workflow execution engine
│   │   ├── communication.rs # Message bus for agents
│   │   └── scheduler.rs     # Task scheduling and assignment
│   ├── ui/                  # UI components
│   │   ├── orchestrator.rs  # Main orchestrator view
│   │   ├── agent_manager.rs # Agent creation and management
│   │   └── workflow_editor.rs # Visual workflow builder
│   └── utils/               # Utilities
│       └── ipc.rs           # IPC client for Tauri backend
```

## Key Features

### 1. Multi-Role Agent System
- **Predefined Roles**: Orchestrator, Architect, Developer, Reviewer, Tester, Documentation Writer, DevOps
- **Custom Roles**: Create agents with custom capabilities
- **Role-Based Task Assignment**: Automatically match tasks to agents based on their expertise

### 2. Visual Workflow Editor
- Drag-and-drop node-based workflow creation
- Support for parallel execution, conditionals, and loops
- Real-time workflow validation
- Template library for common patterns

### 3. Orchestration Engine
- Distributed task execution across multiple agents
- Inter-agent communication via message bus
- Priority-based task scheduling
- Automatic retry and error handling

### 4. Real-Time Monitoring
- Live agent status tracking
- Message flow visualization
- Performance metrics per agent
- Token usage and cost tracking

## Building and Running

### Prerequisites
- Rust 1.75+
- The existing Tauri backend must be running

### Development
```bash
cd src-egui
cargo run
```

### Release Build
```bash
cargo build --release
```

## Integration with Tauri Backend

The egui frontend communicates with the existing Tauri backend via HTTP IPC. Ensure the Tauri backend is running on port 1420 (default).

## Agent Role Examples

### Orchestrator
```rust
AgentRole::Orchestrator {
    subordinates: vec![architect_id, dev1_id, dev2_id],
    max_parallel_tasks: 3,
}
```

### Developer
```rust
AgentRole::Developer {
    language_expertise: vec![ProgrammingLanguage::Rust, ProgrammingLanguage::TypeScript],
    framework_expertise: vec![Framework::Tokio, Framework::React],
    years_experience: 5.0,
}
```

### Custom
```rust
AgentRole::Custom {
    name: "Security Auditor".to_string(),
    capabilities: vec!["penetration_testing", "code_review", "vulnerability_assessment"],
    custom_prompt: "You are a security expert who identifies and fixes vulnerabilities.",
}
```

## Workflow Definition Example

```rust
let web_app_workflow = Workflow {
    name: "Full Stack Web App",
    stages: vec![
        // Architecture Design
        WorkflowNode::Task(WorkflowTask {
            name: "Design Architecture",
            agent_role: AgentRole::Architect,
            inputs: vec!["requirements.md"],
            outputs: vec!["architecture.md", "api_spec.yaml"],
        }),
        // Parallel Development
        WorkflowNode::Parallel(vec![
            WorkflowTask {
                name: "Backend Development",
                agent_role: AgentRole::Developer,
                inputs: vec!["api_spec.yaml"],
                outputs: vec!["backend/"],
            },
            WorkflowTask {
                name: "Frontend Development",
                agent_role: AgentRole::Developer,
                inputs: vec!["api_spec.yaml"],
                outputs: vec!["frontend/"],
            },
        ]),
        // Testing
        WorkflowNode::Task(WorkflowTask {
            name: "Integration Testing",
            agent_role: AgentRole::Tester,
            inputs: vec!["backend/", "frontend/"],
            outputs: vec!["test_report.md"],
        }),
    ],
};
```

## Future Enhancements

1. **Agent Templates**: Pre-configured agents for common roles
2. **Workflow Marketplace**: Share and import workflow templates
3. **Advanced Scheduling**: Resource-aware scheduling, agent affinity
4. **Monitoring Dashboard**: Historical metrics, performance analytics
5. **Plugin System**: Extend agent capabilities with plugins