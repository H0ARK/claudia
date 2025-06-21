# DAG Visualizer Component

A comprehensive React component for visualizing Directed Acyclic Graphs (DAGs) in the orchestration UI, built with react-flow.

## Features

### 1. Graph Visualization
- **Interactive nodes and edges**: Zoom, pan, and select nodes
- **Real-time updates**: WebSocket integration for live task status updates
- **Status color coding**:
  - Pending: Gray
  - Ready: Purple
  - Running: Blue (with animated edges)
  - Completed: Green
  - Failed: Red
  - Cancelled: Yellow
  - Blocked: Pink

### 2. Node Features
- **Task information**: Name, ID, description
- **Agent type icons**: Different icons for orchestrator, planner, worker, etc.
- **Progress indicators**: Real-time progress bars for running tasks
- **Status badges**: Visual status indicators
- **Click interaction**: Select nodes to view detailed information

### 3. Edge Features
- **Dependency arrows**: Clear visual representation of task dependencies
- **Animated flow**: Active paths show animated edges for running tasks
- **Smart routing**: Smooth step edges with proper arrow markers

### 4. Layout Options
- **Horizontal layout**: Tasks flow from left to right
- **Vertical layout**: Tasks flow from top to bottom
- **Radial layout**: Tasks arranged in concentric circles
- **Automatic positioning**: Topological sort-based positioning

### 5. Controls
- **Zoom controls**: Zoom in/out with buttons or scroll
- **Fit to view**: Automatically fit all nodes in viewport
- **Refresh data**: Manual refresh button
- **Layout switcher**: Change between layout modes
- **Status filter**: Filter nodes by status

### 6. Performance
- **Virtualization**: Efficiently handles 100+ nodes
- **Optimized rendering**: Only re-renders changed nodes
- **Smart updates**: Batch updates for better performance

## Usage

```tsx
import { DagVisualizer } from './components/orchestration/DagVisualizer';

function OrchestrationView() {
  const orchestrationId = 123; // Your orchestration ID
  
  return (
    <div className="h-screen">
      <DagVisualizer 
        orchestrationId={orchestrationId}
        className="h-full"
      />
    </div>
  );
}
```

## Backend Integration

The component requires the following Tauri commands:

### `get_orchestration_tasks`
Returns all tasks for a given orchestration ID:
```rust
#[tauri::command]
pub async fn get_orchestration_tasks(
    orchestration_id: i64
) -> Result<Vec<Task>, String>
```

### `select_task`
Emits a task-selected event when a node is clicked:
```rust
#[tauri::command]
pub async fn select_task(task_id: i64) -> Result<(), String>
```

## WebSocket Events

The component listens for real-time updates via the `task-update` event:

```typescript
interface TaskUpdate {
  taskId: number;
  status: string;
  progress?: number;
}
```

## Customization

### Custom Node Types
You can extend the node types by modifying the `agentIcons` object:

```typescript
const agentIcons = {
  orchestrator: GitBranch,
  planner: Layout,
  worker: Cpu,
  // Add your custom types here
};
```

### Custom Status Colors
Modify the `statusColors` object to change the color scheme:

```typescript
const statusColors = {
  pending: '#6b7280',
  running: '#3b82f6',
  // Add or modify status colors
};
```

## Example Implementation

See `DagVisualizerExample.tsx` for a complete implementation example that includes:
- Creating new orchestrations
- Task selection and details view
- Status monitoring
- Orchestration controls (schedule, cancel, etc.)

## Dependencies

- `reactflow`: ^11.11.4
- `@tauri-apps/api`: For backend communication
- `lucide-react`: For icons
- UI components from the project's UI library

## Performance Tips

1. **Large Graphs**: For graphs with 100+ nodes, consider:
   - Using the status filter to show only relevant nodes
   - Implementing pagination or lazy loading
   - Using the fit-to-view feature sparingly

2. **Real-time Updates**: To optimize WebSocket updates:
   - Batch multiple updates when possible
   - Use React.memo for custom node components
   - Throttle rapid status changes

3. **Layout Performance**: The radial layout is most performant for large graphs as it distributes nodes evenly.