# Testing Guide for Claudia Orchestrator

## Overview
This guide explains how to build and test the new egui-based orchestrator system for Claudia.

## Prerequisites
- Rust 1.75+ installed
- Visual Studio Build Tools (for Windows)
- Git

## Building the Project

### Option 1: Using the build script
```batch
build_test.bat
```

### Option 2: Manual build
```bash
# Build entire workspace
cargo build --workspace

# Build Tauri backend only
cd src-tauri
cargo build

# Build egui frontend only
cd src-egui
cargo build
```

## Running the Applications

### Running the egui Orchestrator
```batch
run_egui.bat
```

Or manually:
```bash
cd src-egui
cargo run
```

### Running the Tauri Application
```bash
cd src-tauri
cargo tauri dev
```

## Testing Features

### 1. Agent Management
1. Launch the egui app
2. Navigate to the "Agents" tab
3. Click "Create Agent" to create a new agent
4. Test different agent roles:
   - Orchestrator
   - Developer
   - Architect
   - Reviewer
   - Tester
   - Custom

### 2. Workflow Editor
1. Navigate to the "Workflows" tab
2. Click "New Workflow"
3. Drag and drop nodes from the palette:
   - Task nodes
   - Parallel execution nodes
   - Conditional nodes
   - Loop nodes
4. Connect nodes by dragging between connection points
5. Click on nodes to edit properties

### 3. Orchestration Engine
1. Create multiple agents with different roles
2. Design a workflow using the editor
3. Click "Execute" to run the workflow
4. Monitor agent status in real-time
5. View message logs and metrics

### 4. Inter-Agent Communication
- Agents communicate through the message bus
- Messages are displayed in the message log
- Different message types have different colors/icons

## Known Limitations on Windows

1. **Sandboxing**: The gaol-based sandboxing is disabled on Windows. All sandbox-related features are no-ops.
2. **IPC**: The egui frontend currently expects the Tauri backend to be running on port 1420.

## Troubleshooting

### Build Errors

1. **gaol compilation errors**: These should be resolved as gaol is only compiled on Unix systems.

2. **Missing dependencies**: 
   ```bash
   cargo update
   ```

3. **egui compilation issues**:
   - Ensure you have the latest Rust version
   - Check that all system dependencies for egui are installed

### Runtime Issues

1. **IPC Connection Failed**: 
   - Ensure the Tauri backend is running
   - Check that port 1420 is not blocked

2. **Agent Creation Fails**:
   - Check the console for error messages
   - Ensure the backend database is properly initialized

## Architecture Components

### Models (`src-egui/src/models/`)
- `agent.rs`: Agent roles and capabilities
- `workflow.rs`: Workflow graph structures
- `message.rs`: Inter-agent communication

### Orchestration (`src-egui/src/orchestration/`)
- `engine.rs`: Core workflow execution
- `communication.rs`: Message bus implementation
- `scheduler.rs`: Task scheduling logic

### UI (`src-egui/src/ui/`)
- `orchestrator.rs`: Main dashboard
- `agent_manager.rs`: Agent CRUD operations
- `workflow_editor.rs`: Visual workflow builder

## Next Steps

1. **Integration Testing**: Test the full workflow from agent creation to execution
2. **Performance Testing**: Create workflows with many agents and tasks
3. **UI Polish**: Refine the visual design and user experience
4. **Backend Integration**: Complete the IPC integration with the Tauri backend