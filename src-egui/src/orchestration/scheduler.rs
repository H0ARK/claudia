use crate::models::{AgentId, Task, Priority};
use anyhow::Result;
use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use std::collections::{BinaryHeap, HashMap};
use std::sync::Arc;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct ScheduledTask {
    pub task: Task,
    pub priority: Priority,
    pub scheduled_at: DateTime<Utc>,
    pub assigned_to: Option<AgentId>,
    pub retry_count: u32,
    pub max_retries: u32,
}

impl PartialEq for ScheduledTask {
    fn eq(&self, other: &Self) -> bool {
        self.task.id == other.task.id
    }
}

impl Eq for ScheduledTask {}

impl PartialOrd for ScheduledTask {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ScheduledTask {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Higher priority first, then earlier scheduled time
        match self.priority.cmp(&other.priority).reverse() {
            std::cmp::Ordering::Equal => self.scheduled_at.cmp(&other.scheduled_at),
            other => other,
        }
    }
}

pub struct TaskScheduler {
    queue: Arc<RwLock<BinaryHeap<ScheduledTask>>>,
    task_assignments: Arc<RwLock<HashMap<Uuid, AgentId>>>,
    agent_workload: Arc<RwLock<HashMap<AgentId, Vec<Uuid>>>>,
}

impl TaskScheduler {
    pub fn new() -> Self {
        Self {
            queue: Arc::new(RwLock::new(BinaryHeap::new())),
            task_assignments: Arc::new(RwLock::new(HashMap::new())),
            agent_workload: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn schedule_task(&self, task: Task, priority: Priority) -> Result<()> {
        let scheduled_task = ScheduledTask {
            task,
            priority,
            scheduled_at: Utc::now(),
            assigned_to: None,
            retry_count: 0,
            max_retries: 3,
        };

        self.queue.write().push(scheduled_task);
        Ok(())
    }

    pub fn assign_task(&self, task_id: Uuid, agent_id: AgentId) -> Result<()> {
        self.task_assignments.write().insert(task_id, agent_id.clone());
        
        let mut workload = self.agent_workload.write();
        workload
            .entry(agent_id)
            .or_insert_with(Vec::new)
            .push(task_id);

        Ok(())
    }

    pub fn complete_task(&self, task_id: Uuid) -> Result<()> {
        if let Some(agent_id) = self.task_assignments.write().remove(&task_id) {
            if let Some(tasks) = self.agent_workload.write().get_mut(&agent_id) {
                tasks.retain(|&id| id != task_id);
            }
        }
        Ok(())
    }

    pub fn get_next_task(&self) -> Option<ScheduledTask> {
        self.queue.write().pop()
    }

    pub fn get_agent_workload(&self, agent_id: &AgentId) -> Vec<Uuid> {
        self.agent_workload
            .read()
            .get(agent_id)
            .cloned()
            .unwrap_or_default()
    }

    pub fn reschedule_task(&self, mut task: ScheduledTask) -> Result<()> {
        task.retry_count += 1;
        task.scheduled_at = Utc::now() + chrono::Duration::seconds(
            (2_u32.pow(task.retry_count) * 5) as i64
        );
        
        if task.retry_count <= task.max_retries {
            self.queue.write().push(task);
            Ok(())
        } else {
            Err(anyhow::anyhow!("Task exceeded max retries"))
        }
    }

    pub fn get_queue_size(&self) -> usize {
        self.queue.read().len()
    }

    pub fn get_scheduled_tasks(&self) -> Vec<ScheduledTask> {
        let queue = self.queue.read();
        let mut tasks: Vec<_> = queue.iter().cloned().collect();
        tasks.sort();
        tasks
    }

    pub fn cancel_task(&self, task_id: Uuid) -> Result<()> {
        let mut queue = self.queue.write();
        let tasks: Vec<_> = queue.drain().filter(|t| t.task.id != task_id).collect();
        for task in tasks {
            queue.push(task);
        }
        
        self.complete_task(task_id)?;
        Ok(())
    }

    pub fn rebalance_workload(&self, agents: &[AgentId]) -> Result<()> {
        // Simple round-robin rebalancing
        let mut queue = self.queue.write();
        let mut tasks: Vec<_> = queue.drain().collect();
        
        if !agents.is_empty() {
            for (i, task) in tasks.iter_mut().enumerate() {
                if task.assigned_to.is_none() {
                    task.assigned_to = Some(agents[i % agents.len()].clone());
                }
            }
        }
        
        for task in tasks {
            queue.push(task);
        }
        
        Ok(())
    }
}