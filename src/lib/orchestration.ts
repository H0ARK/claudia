import { invoke } from '@tauri-apps/api/core';
import { listen, UnlistenFn, Event as TauriEvent } from '@tauri-apps/api/event';

// Types matching the Rust structures
export enum TaskStatus {
  Pending = 'pending',
  Ready = 'ready',
  Running = 'running',
  Completed = 'completed',
  Failed = 'failed',
  Cancelled = 'cancelled',
  Blocked = 'blocked',
}

export interface Task {
  id: number;
  orchestration_id: number;
  name: string;
  description: string;
  goal: string;
  agent_config: AgentConfig;
  status: TaskStatus;
  created_at: string;
  started_at?: string;
  completed_at?: string;
  output?: string;
  error?: string;
  retry_count: number;
  max_retries: number;
  depends_on: number[];
}

export interface AgentConfig {
  agent_type: string;
  model: string;
  system_prompt: string;
  max_tokens?: number;
  temperature?: number;
  capabilities: string[];
  sandbox_profile?: string;
}

export interface Artifact {
  name: string;
  artifact_type: string;
  path: string;
  description: string;
  metadata: Record<string, any>;
}

// Event types
export type OrchestrationEventType = 
  | 'taskStatusChanged'
  | 'taskMessage'
  | 'taskProgress'
  | 'taskLog'
  | 'taskArtifacts'
  | 'orchestrationStatusChanged'
  | 'taskAdded'
  | 'taskDependencyResolved'
  | 'orchestrationError';

export interface OrchestrationEvent {
  type: OrchestrationEventType;
  orchestration_id: number;
  timestamp: string;
}

export interface TaskStatusChangedEvent extends OrchestrationEvent {
  type: 'taskStatusChanged';
  task_id: number;
  old_status: TaskStatus;
  new_status: TaskStatus;
}

export interface TaskMessageEvent extends OrchestrationEvent {
  type: 'taskMessage';
  task_id: number;
  message_type: string;
  message: string;
}

export interface TaskProgressEvent extends OrchestrationEvent {
  type: 'taskProgress';
  task_id: number;
  progress: number;
  message: string;
}

export interface TaskLogEvent extends OrchestrationEvent {
  type: 'taskLog';
  task_id: number;
  level: string;
  message: string;
}

export interface TaskArtifactsEvent extends OrchestrationEvent {
  type: 'taskArtifacts';
  task_id: number;
  artifacts: Artifact[];
}

export interface OrchestrationStatusChangedEvent extends OrchestrationEvent {
  type: 'orchestrationStatusChanged';
  status: string;
  total_tasks: number;
  completed_tasks: number;
  failed_tasks: number;
  progress: number;
}

export interface TaskAddedEvent extends OrchestrationEvent {
  type: 'taskAdded';
  task: Task;
}

export interface TaskDependencyResolvedEvent extends OrchestrationEvent {
  type: 'taskDependencyResolved';
  task_id: number;
  dependency_id: number;
}

export interface OrchestrationErrorEvent extends OrchestrationEvent {
  type: 'orchestrationError';
  task_id?: number;
  error: string;
  recoverable: boolean;
}

export type OrchestrationEventData = 
  | TaskStatusChangedEvent
  | TaskMessageEvent
  | TaskProgressEvent
  | TaskLogEvent
  | TaskArtifactsEvent
  | OrchestrationStatusChangedEvent
  | TaskAddedEvent
  | TaskDependencyResolvedEvent
  | OrchestrationErrorEvent;

// Orchestration API
export class OrchestrationClient {
  private listeners: Map<string, UnlistenFn> = new Map();
  private reconnectTimer?: number;
  private isConnected: boolean = false;
  private eventHandlers: Map<string, Set<(event: OrchestrationEventData) => void>> = new Map();

  /**
   * Initialize the orchestration kernel
   */
  async init(): Promise<string> {
    return await invoke('init_orchestration_kernel');
  }

  /**
   * Create a new orchestration with a root goal
   */
  async createOrchestration(rootGoal: string): Promise<number> {
    return await invoke('create_orchestration', { rootGoal });
  }

  /**
   * Add a task to an orchestration
   */
  async addTask(params: {
    orchestrationId: number;
    name: string;
    description: string;
    goal: string;
    agentType: string;
    model: string;
    dependsOn: number[];
  }): Promise<number> {
    return await invoke('add_orchestration_task', params);
  }

  /**
   * Get orchestration status
   */
  async getOrchestrationStatus(orchestrationId: number): Promise<any> {
    return await invoke('get_orchestration_status', { orchestrationId });
  }

  /**
   * Get all tasks for an orchestration
   */
  async getOrchestrationTasks(orchestrationId: number): Promise<Task[]> {
    return await invoke('get_orchestration_tasks', { orchestrationId });
  }

  /**
   * Schedule tasks for execution
   */
  async scheduleTasks(): Promise<string> {
    return await invoke('schedule_orchestration_tasks');
  }

  /**
   * Get ready tasks
   */
  async getReadyTasks(): Promise<Task[]> {
    return await invoke('get_ready_tasks');
  }

  /**
   * Update task status
   */
  async updateTaskStatus(taskId: number, status: TaskStatus): Promise<void> {
    return await invoke('update_task_status', { taskId, status });
  }

  /**
   * Cancel an orchestration
   */
  async cancelOrchestration(orchestrationId: number): Promise<void> {
    return await invoke('cancel_orchestration', { orchestrationId });
  }

  /**
   * Subscribe to orchestration events for a specific orchestration
   */
  async subscribeToOrchestration(
    orchestrationId: number,
    handlers: {
      onTaskStatusChanged?: (event: TaskStatusChangedEvent) => void;
      onTaskMessage?: (event: TaskMessageEvent) => void;
      onTaskProgress?: (event: TaskProgressEvent) => void;
      onTaskLog?: (event: TaskLogEvent) => void;
      onTaskArtifacts?: (event: TaskArtifactsEvent) => void;
      onOrchestrationStatusChanged?: (event: OrchestrationStatusChangedEvent) => void;
      onTaskAdded?: (event: TaskAddedEvent) => void;
      onTaskDependencyResolved?: (event: TaskDependencyResolvedEvent) => void;
      onOrchestrationError?: (event: OrchestrationErrorEvent) => void;
      onAnyEvent?: (event: OrchestrationEventData) => void;
    }
  ): Promise<UnlistenFn> {
    const channel = `orchestration:${orchestrationId}`;
    
    // Set up event routing
    const eventHandler = (event: TauriEvent<OrchestrationEventData>) => {
      const data = event.payload;
      
      // Call specific handler based on event type
      switch (data.type) {
        case 'taskStatusChanged':
          handlers.onTaskStatusChanged?.(data as TaskStatusChangedEvent);
          break;
        case 'taskMessage':
          handlers.onTaskMessage?.(data as TaskMessageEvent);
          break;
        case 'taskProgress':
          handlers.onTaskProgress?.(data as TaskProgressEvent);
          break;
        case 'taskLog':
          handlers.onTaskLog?.(data as TaskLogEvent);
          break;
        case 'taskArtifacts':
          handlers.onTaskArtifacts?.(data as TaskArtifactsEvent);
          break;
        case 'orchestrationStatusChanged':
          handlers.onOrchestrationStatusChanged?.(data as OrchestrationStatusChangedEvent);
          break;
        case 'taskAdded':
          handlers.onTaskAdded?.(data as TaskAddedEvent);
          break;
        case 'taskDependencyResolved':
          handlers.onTaskDependencyResolved?.(data as TaskDependencyResolvedEvent);
          break;
        case 'orchestrationError':
          handlers.onOrchestrationError?.(data as OrchestrationErrorEvent);
          break;
      }
      
      // Always call the generic handler if provided
      handlers.onAnyEvent?.(data);
    };

    // Listen to the specific orchestration channel
    const unlisten = await listen<OrchestrationEventData>(channel, eventHandler);
    this.listeners.set(channel, unlisten);
    
    return unlisten;
  }

  /**
   * Subscribe to all orchestration events
   */
  async subscribeToAllOrchestrations(
    handler: (event: OrchestrationEventData) => void
  ): Promise<UnlistenFn> {
    const channel = 'orchestration:all';
    
    const unlisten = await listen<OrchestrationEventData>(channel, (event) => {
      handler(event.payload);
    });
    
    this.listeners.set(channel, unlisten);
    return unlisten;
  }

  /**
   * Unsubscribe from a specific orchestration
   */
  async unsubscribeFromOrchestration(orchestrationId: number): Promise<void> {
    const channel = `orchestration:${orchestrationId}`;
    const unlisten = this.listeners.get(channel);
    
    if (unlisten) {
      unlisten();
      this.listeners.delete(channel);
    }
  }

  /**
   * Unsubscribe from all events
   */
  async unsubscribeAll(): Promise<void> {
    for (const [channel, unlisten] of this.listeners.entries()) {
      unlisten();
    }
    this.listeners.clear();
  }

  /**
   * Set up automatic reconnection
   */
  setupReconnection(reconnectInterval: number = 5000): void {
    if (this.reconnectTimer) {
      clearInterval(this.reconnectTimer);
    }

    this.reconnectTimer = window.setInterval(() => {
      if (!this.isConnected) {
        this.reconnect();
      }
    }, reconnectInterval);
  }

  /**
   * Attempt to reconnect
   */
  private async reconnect(): Promise<void> {
    try {
      // Re-establish all listeners
      for (const [channel, handlers] of this.eventHandlers.entries()) {
        const unlisten = await listen<OrchestrationEventData>(channel, (event) => {
          for (const handler of handlers) {
            handler(event.payload);
          }
        });
        this.listeners.set(channel, unlisten);
      }
      
      this.isConnected = true;
    } catch (error) {
      console.error('Failed to reconnect to orchestration events:', error);
      this.isConnected = false;
    }
  }

  /**
   * Clean up resources
   */
  async cleanup(): Promise<void> {
    await this.unsubscribeAll();
    
    if (this.reconnectTimer) {
      clearInterval(this.reconnectTimer);
      this.reconnectTimer = undefined;
    }
  }
}

// Export a singleton instance
export const orchestrationClient = new OrchestrationClient();

// Utility functions
export function isTaskComplete(status: TaskStatus): boolean {
  return status === TaskStatus.Completed || status === TaskStatus.Failed || status === TaskStatus.Cancelled;
}

export function isTaskRunning(status: TaskStatus): boolean {
  return status === TaskStatus.Running;
}

export function isTaskWaiting(status: TaskStatus): boolean {
  return status === TaskStatus.Pending || status === TaskStatus.Ready || status === TaskStatus.Blocked;
}

export function getStatusColor(status: TaskStatus): string {
  switch (status) {
    case TaskStatus.Pending:
      return '#gray';
    case TaskStatus.Ready:
      return '#blue';
    case TaskStatus.Running:
      return '#yellow';
    case TaskStatus.Completed:
      return '#green';
    case TaskStatus.Failed:
      return '#red';
    case TaskStatus.Cancelled:
      return '#orange';
    case TaskStatus.Blocked:
      return '#purple';
    default:
      return '#gray';
  }
}

export function formatTaskDuration(task: Task): string {
  if (!task.started_at) return 'Not started';
  
  const start = new Date(task.started_at);
  const end = task.completed_at ? new Date(task.completed_at) : new Date();
  const duration = end.getTime() - start.getTime();
  
  const seconds = Math.floor(duration / 1000);
  const minutes = Math.floor(seconds / 60);
  const hours = Math.floor(minutes / 60);
  
  if (hours > 0) {
    return `${hours}h ${minutes % 60}m`;
  } else if (minutes > 0) {
    return `${minutes}m ${seconds % 60}s`;
  } else {
    return `${seconds}s`;
  }
}