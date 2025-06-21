import React, { useEffect, useState, useCallback } from 'react';
import {
  orchestrationClient,
  Task,
  TaskStatus,
  OrchestrationEventData,
  TaskStatusChangedEvent,
  TaskProgressEvent,
  TaskMessageEvent,
  TaskLogEvent,
  OrchestrationStatusChangedEvent,
  getStatusColor,
  formatTaskDuration,
  isTaskComplete,
  isTaskRunning,
} from '../../lib/orchestration';
import { Card } from '../ui/card';
import { Badge } from '../ui/badge';
import { Button } from '../ui/button';

interface OrchestrationLiveViewProps {
  orchestrationId: number;
  onTaskSelect?: (taskId: number) => void;
}

interface OrchestrationStatus {
  status: string;
  total_tasks: number;
  completed_tasks: number;
  failed_tasks: number;
  progress: number;
}

interface TaskEvent {
  id: string;
  taskId: number;
  type: string;
  message: string;
  timestamp: string;
  level?: string;
}

export const OrchestrationLiveView: React.FC<OrchestrationLiveViewProps> = ({
  orchestrationId,
  onTaskSelect,
}) => {
  const [tasks, setTasks] = useState<Task[]>([]);
  const [orchestrationStatus, setOrchestrationStatus] = useState<OrchestrationStatus | null>(null);
  const [events, setEvents] = useState<TaskEvent[]>([]);
  const [isSubscribed, setIsSubscribed] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // Load initial data
  const loadOrchestrationData = useCallback(async () => {
    try {
      const [tasksData, statusData] = await Promise.all([
        orchestrationClient.getOrchestrationTasks(orchestrationId),
        orchestrationClient.getOrchestrationStatus(orchestrationId),
      ]);
      
      setTasks(tasksData);
      setOrchestrationStatus(statusData);
    } catch (err) {
      setError(`Failed to load orchestration data: ${err}`);
    }
  }, [orchestrationId]);

  // Subscribe to events
  const subscribeToEvents = useCallback(async () => {
    try {
      await orchestrationClient.subscribeToOrchestration(orchestrationId, {
        onTaskStatusChanged: (event: TaskStatusChangedEvent) => {
          setTasks(prev => prev.map(task => 
            task.id === event.task_id 
              ? { ...task, status: event.new_status }
              : task
          ));
          
          setEvents(prev => [{
            id: `${event.task_id}-${event.timestamp}`,
            taskId: event.task_id,
            type: 'status',
            message: `Task status changed from ${event.old_status} to ${event.new_status}`,
            timestamp: event.timestamp,
          }, ...prev].slice(0, 100)); // Keep last 100 events
        },
        
        onTaskProgress: (event: TaskProgressEvent) => {
          setTasks(prev => prev.map(task => 
            task.id === event.task_id 
              ? { ...task, progress: event.progress }
              : task
          ));
          
          setEvents(prev => [{
            id: `${event.task_id}-progress-${event.timestamp}`,
            taskId: event.task_id,
            type: 'progress',
            message: `${event.progress}% - ${event.message}`,
            timestamp: event.timestamp,
          }, ...prev].slice(0, 100));
        },
        
        onTaskMessage: (event: TaskMessageEvent) => {
          setEvents(prev => [{
            id: `${event.task_id}-msg-${event.timestamp}`,
            taskId: event.task_id,
            type: event.message_type,
            message: event.message,
            timestamp: event.timestamp,
          }, ...prev].slice(0, 100));
        },
        
        onTaskLog: (event: TaskLogEvent) => {
          setEvents(prev => [{
            id: `${event.task_id}-log-${event.timestamp}`,
            taskId: event.task_id,
            type: 'log',
            message: event.message,
            timestamp: event.timestamp,
            level: event.level,
          }, ...prev].slice(0, 100));
        },
        
        onOrchestrationStatusChanged: (event: OrchestrationStatusChangedEvent) => {
          setOrchestrationStatus({
            status: event.status,
            total_tasks: event.total_tasks,
            completed_tasks: event.completed_tasks,
            failed_tasks: event.failed_tasks,
            progress: event.progress,
          });
        },
        
        onTaskAdded: (event) => {
          setTasks(prev => [...prev, event.task]);
        },
        
        onOrchestrationError: (event) => {
          setError(`Orchestration error: ${event.error}`);
          setEvents(prev => [{
            id: `error-${event.timestamp}`,
            taskId: event.task_id || 0,
            type: 'error',
            message: event.error,
            timestamp: event.timestamp,
            level: 'error',
          }, ...prev].slice(0, 100));
        },
      });
      
      setIsSubscribed(true);
    } catch (err) {
      setError(`Failed to subscribe to events: ${err}`);
    }
  }, [orchestrationId]);

  // Initialize on mount
  useEffect(() => {
    loadOrchestrationData();
    subscribeToEvents();
    
    // Set up reconnection
    orchestrationClient.setupReconnection();
    
    return () => {
      orchestrationClient.unsubscribeFromOrchestration(orchestrationId);
    };
  }, [orchestrationId, loadOrchestrationData, subscribeToEvents]);

  const handleCancelOrchestration = async () => {
    try {
      await orchestrationClient.cancelOrchestration(orchestrationId);
    } catch (err) {
      setError(`Failed to cancel orchestration: ${err}`);
    }
  };

  const getTaskStatusBadgeVariant = (status: TaskStatus) => {
    switch (status) {
      case TaskStatus.Completed:
        return 'success';
      case TaskStatus.Failed:
        return 'destructive';
      case TaskStatus.Running:
        return 'default';
      case TaskStatus.Cancelled:
        return 'outline';
      default:
        return 'secondary';
    }
  };

  const getEventIcon = (type: string, level?: string) => {
    if (level === 'error' || type === 'error') return '❌';
    if (level === 'warning') return '⚠️';
    if (type === 'status') return '🔄';
    if (type === 'progress') return '📊';
    if (type === 'log') return '📝';
    if (type === 'output') return '📤';
    if (type === 'completion') return '✅';
    return '📌';
  };

  return (
    <div className="space-y-4">
      {/* Orchestration Status */}
      {orchestrationStatus && (
        <Card className="p-4">
          <div className="flex items-center justify-between mb-4">
            <h2 className="text-xl font-semibold">Orchestration Status</h2>
            <div className="flex items-center gap-2">
              {!isSubscribed && (
                <Badge variant="outline">Connecting...</Badge>
              )}
              <Button
                variant="destructive"
                size="sm"
                onClick={handleCancelOrchestration}
                disabled={orchestrationStatus.status !== 'running'}
              >
                Cancel Orchestration
              </Button>
            </div>
          </div>
          
          <div className="grid grid-cols-4 gap-4">
            <div>
              <p className="text-sm text-muted-foreground">Status</p>
              <p className="text-lg font-medium capitalize">{orchestrationStatus.status}</p>
            </div>
            <div>
              <p className="text-sm text-muted-foreground">Progress</p>
              <div className="flex items-center gap-2">
                <div className="flex-1 bg-secondary rounded-full h-2">
                  <div
                    className="bg-primary rounded-full h-2 transition-all duration-300"
                    style={{ width: `${orchestrationStatus.progress}%` }}
                  />
                </div>
                <span className="text-sm font-medium">{orchestrationStatus.progress.toFixed(1)}%</span>
              </div>
            </div>
            <div>
              <p className="text-sm text-muted-foreground">Tasks</p>
              <p className="text-lg font-medium">
                {orchestrationStatus.completed_tasks} / {orchestrationStatus.total_tasks}
              </p>
            </div>
            <div>
              <p className="text-sm text-muted-foreground">Failed</p>
              <p className="text-lg font-medium text-destructive">
                {orchestrationStatus.failed_tasks}
              </p>
            </div>
          </div>
        </Card>
      )}

      {/* Error Display */}
      {error && (
        <Card className="p-4 border-destructive">
          <p className="text-destructive">{error}</p>
        </Card>
      )}

      <div className="grid grid-cols-2 gap-4">
        {/* Tasks List */}
        <Card className="p-4">
          <h3 className="text-lg font-semibold mb-4">Tasks</h3>
          <div className="space-y-2 max-h-96 overflow-y-auto">
            {tasks.map(task => (
              <div
                key={task.id}
                className="p-3 border rounded-lg cursor-pointer hover:bg-secondary/50 transition-colors"
                onClick={() => onTaskSelect?.(task.id)}
              >
                <div className="flex items-center justify-between mb-1">
                  <h4 className="font-medium">{task.name}</h4>
                  <Badge variant={getTaskStatusBadgeVariant(task.status)}>
                    {task.status}
                  </Badge>
                </div>
                <p className="text-sm text-muted-foreground">{task.description}</p>
                {isTaskRunning(task.status) && task.started_at && (
                  <p className="text-xs text-muted-foreground mt-1">
                    Running for {formatTaskDuration(task)}
                  </p>
                )}
                {isTaskComplete(task.status) && task.started_at && (
                  <p className="text-xs text-muted-foreground mt-1">
                    Duration: {formatTaskDuration(task)}
                  </p>
                )}
              </div>
            ))}
          </div>
        </Card>

        {/* Live Events */}
        <Card className="p-4">
          <h3 className="text-lg font-semibold mb-4">Live Events</h3>
          <div className="space-y-1 max-h-96 overflow-y-auto font-mono text-xs">
            {events.map(event => (
              <div
                key={event.id}
                className="flex items-start gap-2 p-1 hover:bg-secondary/30 rounded"
              >
                <span>{getEventIcon(event.type, event.level)}</span>
                <span className="text-muted-foreground">
                  {new Date(event.timestamp).toLocaleTimeString()}
                </span>
                <span className="flex-1 break-all">{event.message}</span>
              </div>
            ))}
            {events.length === 0 && (
              <p className="text-center text-muted-foreground py-4">
                No events yet...
              </p>
            )}
          </div>
        </Card>
      </div>
    </div>
  );
};