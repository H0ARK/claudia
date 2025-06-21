import { useEffect, useState, useCallback, useRef } from 'react';
import {
  orchestrationClient,
  Task,
  OrchestrationEventData,
  TaskStatusChangedEvent,
  TaskProgressEvent,
  TaskMessageEvent,
  OrchestrationStatusChangedEvent,
} from './orchestration';
import { UnlistenFn } from '@tauri-apps/api/event';

export interface UseOrchestrationOptions {
  autoSubscribe?: boolean;
  reconnectInterval?: number;
  maxEventHistory?: number;
}

export interface UseOrchestrationResult {
  tasks: Task[];
  orchestrationStatus: any | null;
  events: OrchestrationEventData[];
  isConnected: boolean;
  error: string | null;
  subscribe: () => Promise<void>;
  unsubscribe: () => Promise<void>;
  refresh: () => Promise<void>;
}

/**
 * React hook for managing orchestration state and events
 */
export function useOrchestration(
  orchestrationId: number,
  options: UseOrchestrationOptions = {}
): UseOrchestrationResult {
  const {
    autoSubscribe = true,
    reconnectInterval = 5000,
    maxEventHistory = 100,
  } = options;

  const [tasks, setTasks] = useState<Task[]>([]);
  const [orchestrationStatus, setOrchestrationStatus] = useState<any | null>(null);
  const [events, setEvents] = useState<OrchestrationEventData[]>([]);
  const [isConnected, setIsConnected] = useState(false);
  const [error, setError] = useState<string | null>(null);
  
  const unlistenRef = useRef<UnlistenFn | null>(null);

  // Load orchestration data
  const refresh = useCallback(async () => {
    try {
      const [tasksData, statusData] = await Promise.all([
        orchestrationClient.getOrchestrationTasks(orchestrationId),
        orchestrationClient.getOrchestrationStatus(orchestrationId),
      ]);
      
      setTasks(tasksData);
      setOrchestrationStatus(statusData);
      setError(null);
    } catch (err) {
      setError(`Failed to load orchestration data: ${err}`);
    }
  }, [orchestrationId]);

  // Subscribe to orchestration events
  const subscribe = useCallback(async () => {
    try {
      // Unsubscribe if already subscribed
      if (unlistenRef.current) {
        unlistenRef.current();
      }

      const unlisten = await orchestrationClient.subscribeToOrchestration(orchestrationId, {
        onTaskStatusChanged: (event: TaskStatusChangedEvent) => {
          setTasks(prev => prev.map(task => 
            task.id === event.task_id 
              ? { ...task, status: event.new_status }
              : task
          ));
          
          setEvents(prev => [event, ...prev].slice(0, maxEventHistory));
        },
        
        onTaskProgress: (event: TaskProgressEvent) => {
          setEvents(prev => [event, ...prev].slice(0, maxEventHistory));
        },
        
        onTaskMessage: (event: TaskMessageEvent) => {
          // Update task output if it's a completion message
          if (event.message_type === 'completion' || event.message_type === 'output') {
            setTasks(prev => prev.map(task => 
              task.id === event.task_id 
                ? { ...task, output: event.message }
                : task
            ));
          }
          
          setEvents(prev => [event, ...prev].slice(0, maxEventHistory));
        },
        
        onOrchestrationStatusChanged: (event: OrchestrationStatusChangedEvent) => {
          setOrchestrationStatus({
            status: event.status,
            total_tasks: event.total_tasks,
            completed_tasks: event.completed_tasks,
            failed_tasks: event.failed_tasks,
            progress: event.progress,
          });
          
          setEvents(prev => [event, ...prev].slice(0, maxEventHistory));
        },
        
        onTaskAdded: (event) => {
          setTasks(prev => [...prev, event.task]);
          setEvents(prev => [event, ...prev].slice(0, maxEventHistory));
        },
        
        onTaskDependencyResolved: (event) => {
          setEvents(prev => [event, ...prev].slice(0, maxEventHistory));
        },
        
        onOrchestrationError: (event) => {
          setError(`Orchestration error: ${event.error}`);
          setEvents(prev => [event, ...prev].slice(0, maxEventHistory));
        },
        
        onAnyEvent: (event) => {
          // Update connection status
          setIsConnected(true);
        },
      });
      
      unlistenRef.current = unlisten;
      setIsConnected(true);
      setError(null);
    } catch (err) {
      setError(`Failed to subscribe to events: ${err}`);
      setIsConnected(false);
    }
  }, [orchestrationId, maxEventHistory]);

  // Unsubscribe from events
  const unsubscribe = useCallback(async () => {
    if (unlistenRef.current) {
      unlistenRef.current();
      unlistenRef.current = null;
    }
    setIsConnected(false);
  }, []);

  // Set up subscriptions and reconnection
  useEffect(() => {
    // Initial data load
    refresh();

    // Subscribe if auto-subscribe is enabled
    if (autoSubscribe) {
      subscribe();
    }

    // Set up reconnection
    orchestrationClient.setupReconnection(reconnectInterval);

    // Cleanup on unmount
    return () => {
      unsubscribe();
    };
  }, [orchestrationId, autoSubscribe, reconnectInterval, refresh, subscribe, unsubscribe]);

  return {
    tasks,
    orchestrationStatus,
    events,
    isConnected,
    error,
    subscribe,
    unsubscribe,
    refresh,
  };
}

/**
 * React hook for subscribing to all orchestrations
 */
export function useAllOrchestrations(
  options: Omit<UseOrchestrationOptions, 'autoSubscribe'> = {}
): {
  events: OrchestrationEventData[];
  isConnected: boolean;
  error: string | null;
} {
  const { reconnectInterval = 5000, maxEventHistory = 100 } = options;

  const [events, setEvents] = useState<OrchestrationEventData[]>([]);
  const [isConnected, setIsConnected] = useState(false);
  const [error, setError] = useState<string | null>(null);
  
  const unlistenRef = useRef<UnlistenFn | null>(null);

  useEffect(() => {
    const subscribe = async () => {
      try {
        const unlisten = await orchestrationClient.subscribeToAllOrchestrations((event) => {
          setEvents(prev => [event, ...prev].slice(0, maxEventHistory));
          setIsConnected(true);
        });
        
        unlistenRef.current = unlisten;
        setIsConnected(true);
        setError(null);
      } catch (err) {
        setError(`Failed to subscribe to all orchestrations: ${err}`);
        setIsConnected(false);
      }
    };

    subscribe();
    orchestrationClient.setupReconnection(reconnectInterval);

    return () => {
      if (unlistenRef.current) {
        unlistenRef.current();
      }
    };
  }, [reconnectInterval, maxEventHistory]);

  return {
    events,
    isConnected,
    error,
  };
}