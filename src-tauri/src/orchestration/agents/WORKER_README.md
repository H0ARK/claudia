# Worker Agent Management System

The Worker Agent Management System provides a comprehensive framework for spawning, managing, and monitoring different types of specialized AI agents within the Claudia orchestration framework.

## Overview

The system is built around the `WorkerManager` struct which handles:
- Spawning different types of worker agents (Developer, Tester, Reviewer, Documentation, Custom)
- Managing agent lifecycle (start, monitor, stop, restart)
- Routing messages between kernel and agents
- Handling agent failures and automatic recovery
- Collecting and aggregating metrics

## Worker Types

### Built-in Worker Types

1. **DeveloperAgent**: Specialized in code implementation
   - Writes clean, efficient code
   - Follows best practices and design patterns
   - Implements error handling and validation
   - Default model: claude-3-5-sonnet-20241022

2. **TesterAgent**: Focused on test writing and execution
   - Creates comprehensive test plans
   - Writes unit, integration, and end-to-end tests
   - Performs coverage analysis
   - Default model: claude-3-5-sonnet-20241022

3. **ReviewerAgent**: Performs code review and analysis
   - Assesses code quality and structure
   - Identifies security vulnerabilities
   - Suggests performance optimizations
   - Default model: claude-3-5-haiku-20241022

4. **DocumentationAgent**: Generates documentation
   - Creates API documentation
   - Writes user guides and tutorials
   - Generates architecture diagrams
   - Default model: claude-3-5-haiku-20241022

5. **CustomAgent**: User-defined agents
   - Configurable for specific tasks
   - Custom system prompts and capabilities

## Core Components

### WorkerManager

The main orchestrator for all worker agents:

```rust
let worker_manager = WorkerManager::new("/path/to/claude".to_string());
```

### WorkerHandle

A handle to interact with a running worker:

```rust
pub struct WorkerHandle {
    pub id: String,
    pub worker_type: WorkerType,
    pub task_id: i64,
    pub status: Arc<RwLock<WorkerStatus>>,
    pub started_at: DateTime<Utc>,
    pub last_heartbeat: Arc<RwLock<DateTime<Utc>>>,
    pub metrics: Arc<RwLock<WorkerMetrics>>,
}
```

### WorkerConfig

Configuration for worker agents:

```rust
pub struct WorkerConfig {
    pub worker_type: WorkerType,
    pub model: Option<String>,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
    pub custom_system_prompt: Option<String>,
    pub additional_capabilities: Vec<String>,
    pub sandbox_profile: Option<String>,
    pub environment_vars: HashMap<String, String>,
    pub working_directory: Option<PathBuf>,
}
```

## Key Methods

### Spawning Workers

```rust
// Spawn with default configuration
let handle = worker_manager.spawn_worker(&task, WorkerType::Developer, None).await?;

// Spawn with custom configuration
let config = WorkerConfig {
    worker_type: WorkerType::Developer,
    model: Some("claude-3-opus-20240229".to_string()),
    temperature: Some(0.3),
    ..Default::default()
};
let handle = worker_manager.spawn_worker(&task, WorkerType::Developer, Some(config)).await?;
```

### Monitoring Workers

```rust
// Get worker status
let status = worker_manager.monitor_worker(&handle).await?;

// Get worker metrics
let metrics = worker_manager.get_worker_metrics(&handle).await;

// Get all active workers
let workers = worker_manager.get_active_workers().await;

// Get workers by type
let dev_workers = worker_manager.get_workers_by_type(WorkerType::Developer).await;
```

### Managing Workers

```rust
// Send message to worker
worker_manager.send_to_worker(&handle, "message").await?;

// Terminate worker
worker_manager.terminate_worker(&handle).await?;

// Restart worker
let new_handle = worker_manager.restart_worker(&handle).await?;
```

### Custom Workers

```rust
// Register custom worker type
let custom_config = WorkerConfig {
    worker_type: WorkerType::Custom(42),
    custom_system_prompt: Some("You are a specialized agent...".to_string()),
    additional_capabilities: vec!["custom_capability".to_string()],
    ..Default::default()
};
worker_manager.register_custom_worker_type(42, custom_config).await?;
```

## Integration with OrchestrationKernel

The WorkerManager seamlessly integrates with the OrchestrationKernel:

```rust
use crate::orchestration::kernel_worker_integration::IntegratedOrchestrationKernel;

let integrated_kernel = IntegratedOrchestrationKernel::new(
    db_path,
    claude_binary_path,
)?;

// Use integrated functionality
integrated_kernel.spawn_typed_worker(task_id, WorkerType::Developer, None).await?;
let metrics = integrated_kernel.get_orchestration_worker_metrics(orchestration_id).await?;
```

## Worker Lifecycle

1. **Starting**: Worker process is being spawned
2. **Running**: Worker is active and ready
3. **Idle**: Worker is running but not processing tasks
4. **Busy**: Worker is actively processing a task
5. **Failed**: Worker encountered an error
6. **Stopped**: Worker has been terminated

## Metrics and Monitoring

Workers collect various metrics:
- Messages sent/received
- Tasks completed/failed
- Total runtime
- Error count

Global metrics aggregate data across all workers.

## Error Handling

- Automatic heartbeat monitoring (2-minute timeout)
- Configurable retry policies
- Graceful failure handling
- Worker restart capabilities

## Example Usage

See `worker_example.rs` for comprehensive examples including:
- Basic worker orchestration
- Parallel task execution
- Worker failure and recovery
- Custom worker types

## Testing

The system includes comprehensive tests in `worker_test.rs` covering:
- Worker spawning and lifecycle
- Configuration management
- Metrics collection
- Custom worker registration

## Security Considerations

- Workers can be sandboxed using `sandbox_profile`
- Environment variable isolation
- Working directory restrictions
- Controlled capability assignment

## Future Enhancements

Potential improvements:
- Message queue integration for better IPC
- Worker pooling for resource efficiency
- Advanced scheduling algorithms
- Real-time monitoring dashboard
- Distributed worker support