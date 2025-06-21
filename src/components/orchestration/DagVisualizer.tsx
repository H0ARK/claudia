import React, { useEffect, useCallback, useMemo, useState } from 'react';
import ReactFlow, {
  Node,
  Edge,
  Controls,
  Background,
  useNodesState,
  useEdgesState,
  addEdge,
  Connection,
  NodeTypes,
  MarkerType,
  ReactFlowProvider,
  useReactFlow,
  Panel,
  Position,
} from 'reactflow';
import 'reactflow/dist/style.css';
import { Card } from '../ui/card';
import { Badge } from '../ui/badge';
import { Button } from '../ui/button';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '../ui/select';
import { 
  Maximize2, 
  Minimize2, 
  RefreshCw, 
  Filter, 
  Layout,
  Play,
  CheckCircle,
  XCircle,
  Clock,
  Loader2,
  AlertCircle,
  Code,
  FileText,
  Users,
  Database,
  Cpu,
  GitBranch
} from 'lucide-react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

// Types
interface TaskNode {
  id: string;
  orchestrationId: number;
  name: string;
  description: string;
  goal: string;
  agentConfig: {
    agent_type: string;
    model: string;
    capabilities: string[];
  };
  status: 'pending' | 'ready' | 'running' | 'completed' | 'failed' | 'cancelled' | 'blocked';
  progress?: number;
  createdAt: string;
  startedAt?: string;
  completedAt?: string;
  output?: string;
  error?: string;
  dependsOn: string[];
}

interface DagVisualizerProps {
  orchestrationId: number;
  className?: string;
}

// Status colors
const statusColors = {
  pending: '#6b7280',   // gray
  ready: '#8b5cf6',     // purple
  running: '#3b82f6',   // blue
  completed: '#10b981', // green
  failed: '#ef4444',    // red
  cancelled: '#f59e0b', // yellow
  blocked: '#ec4899',   // pink
};

// Agent type icons
const agentIcons = {
  orchestrator: GitBranch,
  planner: Layout,
  worker: Cpu,
  code: Code,
  documentation: FileText,
  collaboration: Users,
  data: Database,
};

// Custom node component
const TaskNodeComponent: React.FC<{ data: TaskNode }> = ({ data }) => {
  const Icon = agentIcons[data.agentConfig.agent_type as keyof typeof agentIcons] || Cpu;
  const statusColor = statusColors[data.status];
  
  return (
    <Card className="min-w-[250px] p-3 border-2 transition-all hover:shadow-lg cursor-pointer"
          style={{ borderColor: statusColor }}>
      <div className="flex items-start justify-between mb-2">
        <div className="flex items-center gap-2">
          <Icon className="w-4 h-4" style={{ color: statusColor }} />
          <h3 className="font-semibold text-sm truncate max-w-[150px]">{data.name}</h3>
        </div>
        <Badge variant="outline" style={{ color: statusColor, borderColor: statusColor }}>
          {data.status}
        </Badge>
      </div>
      
      <p className="text-xs text-gray-600 mb-2 line-clamp-2">{data.description}</p>
      
      {data.progress !== undefined && data.status === 'running' && (
        <div className="mb-2">
          <div className="flex items-center justify-between text-xs mb-1">
            <span>Progress</span>
            <span>{data.progress}%</span>
          </div>
          <div className="w-full bg-gray-200 rounded-full h-1.5">
            <div 
              className="bg-blue-500 h-1.5 rounded-full transition-all"
              style={{ width: `${data.progress}%` }}
            />
          </div>
        </div>
      )}
      
      <div className="flex items-center justify-between text-xs text-gray-500">
        <span>ID: #{data.id}</span>
        <span>{data.agentConfig.model}</span>
      </div>
      
      {data.status === 'running' && (
        <div className="absolute -top-2 -right-2">
          <Loader2 className="w-4 h-4 animate-spin text-blue-500" />
        </div>
      )}
    </Card>
  );
};

const nodeTypes: NodeTypes = {
  task: TaskNodeComponent,
};

// Layout algorithms
const layoutGraph = (nodes: Node[], edges: Edge[], direction: 'horizontal' | 'vertical' | 'radial') => {
  const nodeMap = new Map(nodes.map(n => [n.id, n]));
  const adjList = new Map<string, string[]>();
  const inDegree = new Map<string, number>();
  
  // Build adjacency list and calculate in-degrees
  nodes.forEach(node => {
    adjList.set(node.id, []);
    inDegree.set(node.id, 0);
  });
  
  edges.forEach(edge => {
    adjList.get(edge.source)?.push(edge.target);
    inDegree.set(edge.target, (inDegree.get(edge.target) || 0) + 1);
  });
  
  // Topological sort for layering
  const layers: string[][] = [];
  const queue: string[] = [];
  const visited = new Set<string>();
  
  // Find nodes with no dependencies
  inDegree.forEach((degree, nodeId) => {
    if (degree === 0) queue.push(nodeId);
  });
  
  while (queue.length > 0) {
    const layer: string[] = [];
    const nextQueue: string[] = [];
    
    while (queue.length > 0) {
      const nodeId = queue.shift()!;
      if (visited.has(nodeId)) continue;
      
      visited.add(nodeId);
      layer.push(nodeId);
      
      adjList.get(nodeId)?.forEach(targetId => {
        inDegree.set(targetId, (inDegree.get(targetId) || 0) - 1);
        if (inDegree.get(targetId) === 0) {
          nextQueue.push(targetId);
        }
      });
    }
    
    if (layer.length > 0) layers.push(layer);
    queue.push(...nextQueue);
  }
  
  // Position nodes based on layout direction
  const spacing = { x: 300, y: 150 };
  const updatedNodes = nodes.map(node => {
    const layerIndex = layers.findIndex(layer => layer.includes(node.id));
    const positionInLayer = layers[layerIndex]?.indexOf(node.id) || 0;
    const layerSize = layers[layerIndex]?.length || 1;
    
    let position = { x: 0, y: 0 };
    
    if (direction === 'horizontal') {
      position = {
        x: layerIndex * spacing.x,
        y: (positionInLayer - (layerSize - 1) / 2) * spacing.y,
      };
    } else if (direction === 'vertical') {
      position = {
        x: (positionInLayer - (layerSize - 1) / 2) * spacing.x,
        y: layerIndex * spacing.y,
      };
    } else if (direction === 'radial') {
      const radius = 200 + layerIndex * 150;
      const angle = (positionInLayer / layerSize) * 2 * Math.PI;
      position = {
        x: Math.cos(angle) * radius,
        y: Math.sin(angle) * radius,
      };
    }
    
    return {
      ...node,
      position,
    };
  });
  
  return updatedNodes;
};

// Main component
const DagVisualizerContent: React.FC<DagVisualizerProps> = ({ orchestrationId, className }) => {
  const reactFlowInstance = useReactFlow();
  const [nodes, setNodes, onNodesChange] = useNodesState([]);
  const [edges, setEdges, onEdgesChange] = useEdgesState([]);
  const [layoutDirection, setLayoutDirection] = useState<'horizontal' | 'vertical' | 'radial'>('horizontal');
  const [statusFilter, setStatusFilter] = useState<string>('all');
  const [selectedNode, setSelectedNode] = useState<string | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  
  // Convert task data to ReactFlow nodes and edges
  const convertToFlowElements = useCallback((tasks: TaskNode[]) => {
    const flowNodes: Node[] = tasks.map(task => ({
      id: task.id.toString(),
      type: 'task',
      position: { x: 0, y: 0 },
      data: task,
      sourcePosition: layoutDirection === 'horizontal' ? Position.Right : Position.Bottom,
      targetPosition: layoutDirection === 'horizontal' ? Position.Left : Position.Top,
    }));
    
    const flowEdges: Edge[] = [];
    tasks.forEach(task => {
      task.dependsOn.forEach(depId => {
        flowEdges.push({
          id: `${depId}-${task.id}`,
          source: depId.toString(),
          target: task.id.toString(),
          type: 'smoothstep',
          animated: task.status === 'running',
          style: {
            stroke: task.status === 'running' ? statusColors.running : '#94a3b8',
            strokeWidth: 2,
          },
          markerEnd: {
            type: MarkerType.ArrowClosed,
            color: task.status === 'running' ? statusColors.running : '#94a3b8',
          },
        });
      });
    });
    
    const layoutedNodes = layoutGraph(flowNodes, flowEdges, layoutDirection);
    setNodes(layoutedNodes);
    setEdges(flowEdges);
  }, [layoutDirection, setNodes, setEdges]);
  
  // Load orchestration data
  const loadOrchestrationData = useCallback(async () => {
    try {
      setIsLoading(true);
      const tasks = await invoke<TaskNode[]>('get_orchestration_tasks', { 
        orchestrationId 
      });
      convertToFlowElements(tasks);
    } catch (error) {
      console.error('Failed to load orchestration data:', error);
    } finally {
      setIsLoading(false);
    }
  }, [orchestrationId, convertToFlowElements]);
  
  // Set up WebSocket connection for live updates
  useEffect(() => {
    loadOrchestrationData();
    
    // Listen for task updates
    const unsubscribe = listen<{ taskId: number; status: string; progress?: number }>('task-update', (event) => {
      setNodes(nds => 
        nds.map(node => {
          if (node.id === event.payload.taskId.toString()) {
            return {
              ...node,
              data: {
                ...node.data,
                status: event.payload.status,
                progress: event.payload.progress,
              },
            };
          }
          return node;
        })
      );
      
      // Update edge animations
      setEdges(eds =>
        eds.map(edge => {
          if (edge.target === event.payload.taskId.toString()) {
            return {
              ...edge,
              animated: event.payload.status === 'running',
              style: {
                ...edge.style,
                stroke: event.payload.status === 'running' ? statusColors.running : '#94a3b8',
              },
            };
          }
          return edge;
        })
      );
    });
    
    return () => {
      unsubscribe.then(fn => fn());
    };
  }, [orchestrationId, loadOrchestrationData, setNodes, setEdges]);
  
  // Filter nodes by status
  const filteredNodes = useMemo(() => {
    if (statusFilter === 'all') return nodes;
    return nodes.filter(node => node.data.status === statusFilter);
  }, [nodes, statusFilter]);
  
  // Handlers
  const onConnect = useCallback(
    (params: Connection) => setEdges((eds) => addEdge(params, eds)),
    [setEdges]
  );
  
  const onNodeClick = useCallback((event: React.MouseEvent, node: Node) => {
    setSelectedNode(node.id);
    // You can emit an event here to show task details in a sidebar
    invoke('select_task', { taskId: parseInt(node.id) });
  }, []);
  
  const fitView = useCallback(() => {
    reactFlowInstance.fitView({ padding: 0.2, duration: 800 });
  }, [reactFlowInstance]);
  
  const refreshData = useCallback(() => {
    loadOrchestrationData();
  }, [loadOrchestrationData]);
  
  if (isLoading) {
    return (
      <div className="flex items-center justify-center h-full">
        <Loader2 className="w-8 h-8 animate-spin text-gray-500" />
      </div>
    );
  }
  
  return (
    <div className={`relative w-full h-full ${className}`}>
      <ReactFlow
        nodes={filteredNodes}
        edges={edges}
        onNodesChange={onNodesChange}
        onEdgesChange={onEdgesChange}
        onConnect={onConnect}
        onNodeClick={onNodeClick}
        nodeTypes={nodeTypes}
        fitView
        proOptions={{ hideAttribution: true }}
      >
        <Background variant="dots" gap={12} size={1} />
        <Controls showInteractive={false} />
        
        {/* Custom controls panel */}
        <Panel position="top-left" className="flex gap-2">
          <Card className="p-2">
            <div className="flex items-center gap-2">
              <Select value={layoutDirection} onValueChange={(value: any) => setLayoutDirection(value)}>
                <SelectTrigger className="w-32">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="horizontal">Horizontal</SelectItem>
                  <SelectItem value="vertical">Vertical</SelectItem>
                  <SelectItem value="radial">Radial</SelectItem>
                </SelectContent>
              </Select>
              
              <Button size="sm" variant="outline" onClick={fitView}>
                <Maximize2 className="w-4 h-4" />
              </Button>
              
              <Button size="sm" variant="outline" onClick={refreshData}>
                <RefreshCw className="w-4 h-4" />
              </Button>
            </div>
          </Card>
          
          <Card className="p-2">
            <div className="flex items-center gap-2">
              <Filter className="w-4 h-4 text-gray-500" />
              <Select value={statusFilter} onValueChange={setStatusFilter}>
                <SelectTrigger className="w-32">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="all">All</SelectItem>
                  <SelectItem value="pending">Pending</SelectItem>
                  <SelectItem value="ready">Ready</SelectItem>
                  <SelectItem value="running">Running</SelectItem>
                  <SelectItem value="completed">Completed</SelectItem>
                  <SelectItem value="failed">Failed</SelectItem>
                  <SelectItem value="blocked">Blocked</SelectItem>
                </SelectContent>
              </Select>
            </div>
          </Card>
        </Panel>
        
        {/* Statistics panel */}
        <Panel position="bottom-left" className="bg-white/90 backdrop-blur p-3 rounded-lg shadow-lg">
          <div className="flex items-center gap-4 text-sm">
            <div className="flex items-center gap-1">
              <div className="w-3 h-3 rounded-full" style={{ backgroundColor: statusColors.pending }} />
              <span>Pending: {nodes.filter(n => n.data.status === 'pending').length}</span>
            </div>
            <div className="flex items-center gap-1">
              <div className="w-3 h-3 rounded-full" style={{ backgroundColor: statusColors.running }} />
              <span>Running: {nodes.filter(n => n.data.status === 'running').length}</span>
            </div>
            <div className="flex items-center gap-1">
              <div className="w-3 h-3 rounded-full" style={{ backgroundColor: statusColors.completed }} />
              <span>Completed: {nodes.filter(n => n.data.status === 'completed').length}</span>
            </div>
            <div className="flex items-center gap-1">
              <div className="w-3 h-3 rounded-full" style={{ backgroundColor: statusColors.failed }} />
              <span>Failed: {nodes.filter(n => n.data.status === 'failed').length}</span>
            </div>
          </div>
        </Panel>
      </ReactFlow>
    </div>
  );
};

// Export wrapped in ReactFlowProvider
export const DagVisualizer: React.FC<DagVisualizerProps> = (props) => {
  return (
    <ReactFlowProvider>
      <DagVisualizerContent {...props} />
    </ReactFlowProvider>
  );
};

export default DagVisualizer;