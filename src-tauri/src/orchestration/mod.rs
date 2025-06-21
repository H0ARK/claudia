pub mod kernel;
pub mod protocol;
pub mod file_router;
pub mod agents;
pub mod plugins;
pub mod events;

pub use kernel::{
    OrchestrationKernel,
    TaskStatus,
    AgentConfig,
    Task,
    AgentMessage,
};

pub use protocol::{
    Protocol,
    JsonProtocol,
    ProtocolMessage,
    TaskRequest,
    TaskResult,
    TaskProgress,
    TaskLog,
    TaskContext,
    Artifact,
    ArtifactType,
    LogLevel,
};

pub use file_router::{
    FileRouter,
    ArtifactId,
    MountPoint,
    ArtifactMetadata,
    McpConfig,
};

pub use agents::{
    PlannerAgent,
    PlannerContext,
    TaskPlan,
    PlannedTask,
    AgentType,
    ResourceConstraints,
    ResourceRequirements,
    ExpectedInput,
    ExpectedOutput,
    PlanMetadata,
    RiskMitigation,
};

pub use plugins::{
    Plugin,
    PluginCallback,
    PluginConfig,
    PluginMetadata,
    PluginRegistry,
    PluginSandbox,
    KernelCallback,
};

pub use events::{
    OrchestrationEvent,
    OrchestrationEventEmitter,
    EmitOrchestrationEvent,
};