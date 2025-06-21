# Orchestration Events Documentation

This document describes the real-time event system for orchestration updates in Claudia.

## Overview

The orchestration event system uses Tauri's built-in event system to provide real-time updates from the backend to the frontend. This enables live monitoring of task execution, progress updates, and status changes without the need for polling.

## Architecture

### Backend (Rust)

1. **Event Emitter** (`src-tauri/src/orchestration/events.rs`)
   - `OrchestrationEventEmitter`: Handles event emission to frontend
   - Emits events to both specific orchestration channels and a global channel
   - Automatically converts between different message types

2. **Integration Points**
   - `OrchestrationKernel`: Emits events on status changes, messages, and errors
   - Events are emitted during:
     - Task status changes
     - Agent message handling
     - Task addition
     - Error conditions

### Frontend (TypeScript/React)

1. **Client Library** (`src/lib/orchestration.ts`)
   - `OrchestrationClient`: Main client for interacting with orchestrations
   - Event subscription methods for specific orchestrations or all orchestrations
   - Automatic reconnection support

2. **React Hook** (`src/lib/useOrchestration.ts`)
   - `useOrchestration`: Hook for managing orchestration state and events
   - `useAllOrchestrations`: Hook for monitoring all orchestrations
   - Automatic state management and event history

## Event Types

### TaskStatusChanged
Emitted when a task transitions between states.
```typescript
{
  type: 'taskStatusChanged',
  orchestration_id: number,
  task_id: number,
  old_status: TaskStatus,
  new_status: TaskStatus,
  timestamp: string
}
```

### TaskMessage
General message from a task.
```typescript
{
  type: 'taskMessage',
  orchestration_id: number,
  task_id: number,
  message_type: string,
  message: string,
  timestamp: string
}
```

### TaskProgress
Progress update from a running task.
```typescript
{
  type: 'taskProgress',
  orchestration_id: number,
  task_id: number,
  progress: number, // 0-100
  message: string,
  timestamp: string
}
```

### TaskLog
Log entry from a task.
```typescript
{
  type: 'taskLog',
  orchestration_id: number,
  task_id: number,
  level: string, // debug, info, warning, error
  message: string,
  timestamp: string
}
```

### TaskArtifacts
Notification of artifacts produced by a task.
```typescript
{
  type: 'taskArtifacts',
  orchestration_id: number,
  task_id: number,
  artifacts: Artifact[],
  timestamp: string
}
```

### OrchestrationStatusChanged
Overall orchestration status update.
```typescript
{
  type: 'orchestrationStatusChanged',
  orchestration_id: number,
  status: string,
  total_tasks: number,
  completed_tasks: number,
  failed_tasks: number,
  progress: number,
  timestamp: string
}
```

### TaskAdded
New task added to orchestration.
```typescript
{
  type: 'taskAdded',
  orchestration_id: number,
  task: Task,
  timestamp: string
}
```

### OrchestrationError
Error occurred during orchestration.
```typescript
{
  type: 'orchestrationError',
  orchestration_id: number,
  task_id?: number,
  error: string,
  recoverable: boolean,
  timestamp: string
}
```

## Usage Examples

### Basic Event Subscription

```typescript
import { orchestrationClient } from '@/lib/orchestration';

// Subscribe to a specific orchestration
const unlisten = await orchestrationClient.subscribeToOrchestration(
  orchestrationId,
  {
    onTaskStatusChanged: (event) => {
      console.log(`Task ${event.task_id} changed from ${event.old_status} to ${event.new_status}`);
    },
    onTaskProgress: (event) => {
      console.log(`Task ${event.task_id}: ${event.progress}% - ${event.message}`);
    },
    onOrchestrationError: (event) => {
      console.error(`Error: ${event.error}`);
    }
  }
);

// Later: unsubscribe
unlisten();
```

### Using the React Hook

```typescript
import { useOrchestration } from '@/lib/useOrchestration';

function MyComponent({ orchestrationId }) {
  const {
    tasks,
    orchestrationStatus,
    events,
    isConnected,
    error
  } = useOrchestration(orchestrationId);

  return (
    <div>
      {isConnected ? '🟢 Connected' : '🔴 Disconnected'}
      <h2>Tasks: {tasks.length}</h2>
      <p>Progress: {orchestrationStatus?.progress}%</p>
    </div>
  );
}
```

### Monitoring All Orchestrations

```typescript
import { useAllOrchestrations } from '@/lib/useOrchestration';

function OrchestrationMonitor() {
  const { events, isConnected } = useAllOrchestrations();

  return (
    <div>
      <h2>All Orchestration Events</h2>
      {events.map((event, i) => (
        <div key={i}>
          {event.type} - Orchestration #{event.orchestration_id}
        </div>
      ))}
    </div>
  );
}
```

## Event Channels

Events are emitted on two channels:

1. **Specific Orchestration Channel**: `orchestration:{id}`
   - Only receives events for a specific orchestration
   - Used for focused monitoring of a single orchestration

2. **Global Channel**: `orchestration:all`
   - Receives all orchestration events
   - Used for system-wide monitoring

## Reconnection Support

The client library includes automatic reconnection support:

```typescript
// Set up automatic reconnection with 5-second intervals
orchestrationClient.setupReconnection(5000);
```

This ensures that if the connection is lost, the client will automatically attempt to reconnect and re-establish event subscriptions.

## Best Practices

1. **Unsubscribe on Cleanup**: Always unsubscribe from events when components unmount to prevent memory leaks.

2. **Event History Management**: Limit event history size to prevent excessive memory usage:
   ```typescript
   const { events } = useOrchestration(id, { maxEventHistory: 100 });
   ```

3. **Error Handling**: Always handle potential errors in event handlers:
   ```typescript
   onTaskStatusChanged: (event) => {
     try {
       updateTaskStatus(event);
     } catch (error) {
       console.error('Failed to handle status change:', error);
     }
   }
   ```

4. **Selective Subscriptions**: Only subscribe to events you need to reduce overhead:
   ```typescript
   // Only subscribe to status changes
   subscribeToOrchestration(id, {
     onTaskStatusChanged: handleStatusChange
   });
   ```

## Testing

The system includes example components for testing:

- `OrchestrationLiveView`: Visual component for monitoring orchestrations
- `OrchestrationLiveViewExample`: Complete example with orchestration creation
- Manual event emission via the `send_agent_message` command for testing