use crate::models::{
    Agent, AgentId, AgentStatus, Message, MessageContent, MessageTarget, MessageType,
    Task, TaskResult, TaskStatus, Workflow, WorkflowId, WorkflowNodeType,
};
use crate::orchestration::{MessageBus, TaskScheduler};
use anyhow::Result;
use chrono::Utc;
use dashmap::DashMap;
use petgraph::graph::NodeIndex;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct OrchestrationEngine {
    agents: Arc<DashMap<AgentId, Agent>>,
    workflows: Arc<DashMap<WorkflowId, WorkflowExecution>>,
    message_bus: Arc<MessageBus>,
    scheduler: Arc<TaskScheduler>,
    running: Arc<RwLock<bool>>,
}

struct WorkflowExecution {
    workflow: Workflow,
    state: WorkflowState,
    node_states: HashMap<NodeIndex, NodeExecutionState>,
    variables: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone)]
pub enum WorkflowState {
    Pending,
    Running,
    Paused,
    Completed,
    Failed(String),
}

#[derive(Debug, Clone)]
enum NodeExecutionState {
    Pending,
    Running { started_at: chrono::DateTime<Utc> },
    Completed { result: serde_json::Value },
    Failed { error: String },
    Skipped,
}

impl OrchestrationEngine {
    pub fn new() -> Self {
        let message_bus = Arc::new(MessageBus::new());
        let scheduler = Arc::new(TaskScheduler::new());

        Self {
            agents: Arc::new(DashMap::new()),
            workflows: Arc::new(DashMap::new()),
            message_bus,
            scheduler,
            running: Arc::new(RwLock::new(false)),
        }
    }

    pub async fn start(&self) -> Result<()> {
        *self.running.write().await = true;
        
        // Start background tasks
        self.spawn_message_processor();
        self.spawn_workflow_executor();
        self.spawn_health_monitor();

        Ok(())
    }

    pub async fn stop(&self) -> Result<()> {
        *self.running.write().await = false;
        Ok(())
    }

    pub fn register_agent(&self, agent: Agent) -> Result<()> {
        self.agents.insert(agent.id.clone(), agent);
        Ok(())
    }

    pub fn unregister_agent(&self, agent_id: &AgentId) -> Result<()> {
        self.agents.remove(agent_id);
        Ok(())
    }

    pub async fn execute_workflow(&self, workflow: Workflow) -> Result<WorkflowId> {
        let workflow_id = workflow.id.clone();
        
        // Validate workflow
        workflow.validate().map_err(|e| anyhow::anyhow!(e))?;

        // Initialize execution state
        let execution = WorkflowExecution {
            workflow,
            state: WorkflowState::Pending,
            node_states: HashMap::new(),
            variables: HashMap::new(),
        };

        self.workflows.insert(workflow_id.clone(), execution);
        
        // Start execution
        self.start_workflow_execution(workflow_id.clone()).await?;

        Ok(workflow_id)
    }

    async fn start_workflow_execution(&self, workflow_id: WorkflowId) -> Result<()> {
        if let Some(mut execution) = self.workflows.get_mut(&workflow_id) {
            execution.state = WorkflowState::Running;
            
            // Rebuild graph for traversal
            execution.workflow.graph.rebuild_graph();
            
            // Start from the start node
            let start_nodes: Vec<_> = execution
                .workflow
                .graph
                .graph
                .node_indices()
                .filter(|&idx| {
                    matches!(
                        execution.workflow.graph.graph[idx].node_type,
                        WorkflowNodeType::Start
                    )
                })
                .collect();

            for node in start_nodes {
                self.execute_node(workflow_id.clone(), node).await?;
            }
        }

        Ok(())
    }

    fn execute_node(&self, workflow_id: WorkflowId, node_idx: NodeIndex) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<()>> + Send + '_>> {
        Box::pin(async move {
        let node = {
            let execution = self.workflows.get(&workflow_id)
                .ok_or_else(|| anyhow::anyhow!("Workflow not found"))?;
            execution.workflow.graph.graph[node_idx].clone()
        };

        match node.node_type {
            WorkflowNodeType::Start => {
                // Mark as completed and execute successors
                self.complete_node(workflow_id.clone(), node_idx, serde_json::Value::Null).await?;
            }
            WorkflowNodeType::End => {
                // Workflow completed
                if let Some(mut execution) = self.workflows.get_mut(&workflow_id) {
                    execution.state = WorkflowState::Completed;
                }
            }
            WorkflowNodeType::Task(task) => {
                self.execute_task_node(workflow_id, node_idx, task).await?;
            }
            WorkflowNodeType::Parallel(tasks) => {
                self.execute_parallel_tasks(workflow_id, node_idx, tasks).await?;
            }
            WorkflowNodeType::Conditional { condition, true_branch, false_branch } => {
                self.execute_conditional(workflow_id, node_idx, condition, true_branch, false_branch).await?;
            }
            WorkflowNodeType::Loop { condition, body, max_iterations } => {
                self.execute_loop(workflow_id, node_idx, condition, body, max_iterations).await?;
            }
            WorkflowNodeType::Wait { duration } => {
                tokio::time::sleep(duration.to_std().unwrap()).await;
                self.complete_node(workflow_id, node_idx, serde_json::Value::Null).await?;
            }
        }

        Ok(())
        })
    }

    async fn execute_task_node(
        &self,
        workflow_id: WorkflowId,
        node_idx: NodeIndex,
        task: crate::models::workflow::WorkflowTask,
    ) -> Result<()> {
        // Find suitable agent
        let agent_id = if let Some(id) = task.assigned_agent {
            id
        } else {
            self.find_suitable_agent(&task)?
        };

        // Create task
        let task_obj = Task {
            id: task.id,
            name: task.name,
            description: task.description,
            requirements: HashMap::new(),
            inputs: vec![],
            expected_outputs: vec![],
            deadline: task.timeout.map(|d| Utc::now() + d),
            dependencies: vec![],
        };

        // Send task to agent
        let message = Message::new(
            AgentId::new(), // System agent
            MessageTarget::Agent(agent_id.clone()),
            MessageType::TaskAssignment,
            MessageContent {
                text: format!("Please execute task: {}", task_obj.name),
                structured_data: Some(serde_json::to_value(&task_obj)?),
                attachments: vec![],
            },
        );

        self.message_bus.send(message).await?;

        // Update node state
        if let Some(mut execution) = self.workflows.get_mut(&workflow_id) {
            execution.node_states.insert(
                node_idx,
                NodeExecutionState::Running {
                    started_at: Utc::now(),
                },
            );
        }

        // Update agent status
        if let Some(mut agent) = self.agents.get_mut(&agent_id) {
            agent.status = AgentStatus::Working {
                task_id: task_obj.id,
                progress: 0.0,
            };
        }

        Ok(())
    }

    async fn execute_parallel_tasks(
        &self,
        workflow_id: WorkflowId,
        node_idx: NodeIndex,
        tasks: Vec<crate::models::workflow::WorkflowTask>,
    ) -> Result<()> {
        let mut handles = vec![];

        for task in tasks {
            let engine = self.clone();
            let wf_id = workflow_id.clone();
            let handle = tokio::spawn(async move {
                engine.execute_task_node(wf_id, node_idx, task).await
            });
            handles.push(handle);
        }

        // Wait for all parallel tasks
        for handle in handles {
            handle.await??;
        }

        self.complete_node(workflow_id, node_idx, serde_json::Value::Null).await?;
        Ok(())
    }

    async fn execute_conditional(
        &self,
        workflow_id: WorkflowId,
        node_idx: NodeIndex,
        condition: String,
        _true_branch: Box<crate::models::workflow::WorkflowNode>,
        false_branch: Option<Box<crate::models::workflow::WorkflowNode>>,
    ) -> Result<()> {
        // Evaluate condition (simplified for now)
        let result = self.evaluate_condition(&workflow_id, &condition).await?;

        if result {
            // Execute true branch
            // TODO: Add true branch as temporary node and execute
        } else if let Some(_false_node) = false_branch {
            // Execute false branch
            // TODO: Add false branch as temporary node and execute
        }

        self.complete_node(workflow_id, node_idx, serde_json::Value::Bool(result)).await?;
        Ok(())
    }

    async fn execute_loop(
        &self,
        workflow_id: WorkflowId,
        node_idx: NodeIndex,
        condition: String,
        _body: Box<crate::models::workflow::WorkflowNode>,
        max_iterations: Option<u32>,
    ) -> Result<()> {
        let mut iterations = 0;

        loop {
            if let Some(max) = max_iterations {
                if iterations >= max {
                    break;
                }
            }

            let should_continue = self.evaluate_condition(&workflow_id, &condition).await?;
            if !should_continue {
                break;
            }

            // Execute loop body
            // TODO: Add body as temporary node and execute

            iterations += 1;
        }

        self.complete_node(workflow_id, node_idx, serde_json::Value::Number(iterations.into())).await?;
        Ok(())
    }

    async fn complete_node(
        &self,
        workflow_id: WorkflowId,
        node_idx: NodeIndex,
        result: serde_json::Value,
    ) -> Result<()> {
        // Update node state
        if let Some(mut execution) = self.workflows.get_mut(&workflow_id) {
            execution.node_states.insert(
                node_idx,
                NodeExecutionState::Completed { result },
            );

            // Execute successor nodes
            let successors: Vec<_> = execution
                .workflow
                .graph
                .graph
                .neighbors(node_idx)
                .collect();

            drop(execution); // Release lock before recursive calls

            for successor in successors {
                self.execute_node(workflow_id.clone(), successor).await?;
            }
        }

        Ok(())
    }

    async fn evaluate_condition(&self, _workflow_id: &WorkflowId, condition: &str) -> Result<bool> {
        // Simple condition evaluation - in real implementation, this would be more sophisticated
        // Could use a proper expression evaluator or even ask an LLM agent
        Ok(condition.contains("true") || condition == "1")
    }

    fn find_suitable_agent(&self, _task: &crate::models::workflow::WorkflowTask) -> Result<AgentId> {
        // Find an agent with matching role and idle status
        for agent in self.agents.iter() {
            if agent.status == AgentStatus::Idle {
                // Check if agent role matches task requirements
                // For now, simple role matching
                return Ok(agent.id.clone());
            }
        }

        Err(anyhow::anyhow!("No suitable agent available"))
    }

    fn spawn_message_processor(&self) {
        let engine = self.clone();
        tokio::spawn(async move {
            while *engine.running.read().await {
                // Process messages from agents
                if let Ok(message) = engine.message_bus.receive().await {
                    if let Err(e) = engine.process_message(message).await {
                        log::error!("Error processing message: {:?}", e);
                    }
                }
                tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
            }
        });
    }

    fn spawn_workflow_executor(&self) {
        let engine = self.clone();
        tokio::spawn(async move {
            while *engine.running.read().await {
                // Check for pending workflows and task timeouts
                engine.check_workflows().await;
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            }
        });
    }

    fn spawn_health_monitor(&self) {
        let engine = self.clone();
        tokio::spawn(async move {
            while *engine.running.read().await {
                // Monitor agent health and reassign tasks if needed
                engine.monitor_agent_health().await;
                tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
            }
        });
    }

    async fn process_message(&self, message: Message) -> Result<()> {
        match message.message_type {
            MessageType::StatusUpdate => {
                // Update agent status
                if let Some(mut agent) = self.agents.get_mut(&message.from) {
                    // Parse status from message
                    if let Some(data) = message.content.structured_data {
                        if let Some(progress) = data["progress"].as_f64() {
                            if let AgentStatus::Working { task_id, .. } = &agent.status {
                                agent.status = AgentStatus::Working {
                                    task_id: *task_id,
                                    progress: progress as f32,
                                };
                            }
                        }
                    }
                }
            }
            MessageType::ResultDelivery => {
                // Handle task completion
                if let Some(data) = message.content.structured_data {
                    if let Ok(result) = serde_json::from_value::<TaskResult>(data) {
                        self.handle_task_result(message.from, result).await?;
                    }
                }
            }
            MessageType::Error => {
                // Handle agent errors
                log::error!("Agent error: {:?}", message.content.text);
            }
            _ => {
                // Forward to appropriate handlers
            }
        }

        Ok(())
    }

    async fn handle_task_result(&self, agent_id: AgentId, result: TaskResult) -> Result<()> {
        // Update agent status
        if let Some(mut agent) = self.agents.get_mut(&agent_id) {
            agent.status = AgentStatus::Idle;
            
            // Update metrics
            agent.metrics.total_tasks += 1;
            if result.status == TaskStatus::Completed {
                agent.metrics.successful_tasks += 1;
            } else {
                agent.metrics.failed_tasks += 1;
            }
            agent.metrics.total_tokens_used += result.metrics.tokens_used;
            agent.metrics.total_cost_usd += result.metrics.cost_usd;
        }

        // Find the workflow and node this task belongs to
        // Update workflow execution state
        // This is simplified - in reality we'd track task->node mapping

        Ok(())
    }

    async fn check_workflows(&self) {
        // Check for timeouts, retries, etc.
        for mut workflow_exec in self.workflows.iter_mut() {
            if let WorkflowState::Running = workflow_exec.state {
                // Check if workflow has timed out
                if let Some(timeout) = workflow_exec.workflow.timeout {
                    let elapsed = Utc::now() - workflow_exec.workflow.created_at;
                    if elapsed > timeout {
                        workflow_exec.state = WorkflowState::Failed("Workflow timed out".to_string());
                    }
                }
            }
        }
    }

    async fn monitor_agent_health(&self) {
        for agent in self.agents.iter() {
            // Check if agent is stuck
            if let AgentStatus::Working { task_id, .. } = &agent.status {
                let last_active = agent.last_active;
                let elapsed = Utc::now() - last_active;
                
                if elapsed.num_minutes() > 5 {
                    log::warn!("Agent {} appears to be stuck on task {}", agent.name, task_id);
                    // Could implement task reassignment here
                }
            }
        }
    }

    pub fn get_agents(&self) -> Vec<Agent> {
        self.agents.iter().map(|a| a.value().clone()).collect()
    }

    pub fn get_workflows(&self) -> Vec<(WorkflowId, WorkflowState)> {
        self.workflows
            .iter()
            .map(|w| (w.key().clone(), w.state.clone()))
            .collect()
    }
}

impl Clone for OrchestrationEngine {
    fn clone(&self) -> Self {
        Self {
            agents: self.agents.clone(),
            workflows: self.workflows.clone(),
            message_bus: self.message_bus.clone(),
            scheduler: self.scheduler.clone(),
            running: self.running.clone(),
        }
    }
}