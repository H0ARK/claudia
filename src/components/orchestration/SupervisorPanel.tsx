import React, { useState, useEffect, useCallback, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { Button } from '../ui/button';
import { Input } from '../ui/input';
import { Card, CardContent, CardHeader, CardTitle } from '../ui/card';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '../ui/tabs';
import { Badge } from '../ui/badge';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '../ui/select';
import { 
  Play, 
  Pause, 
  Square, 
  RotateCcw, 
  Search, 
  Filter,
  ChevronRight,
  ChevronDown,
  FileText,
  Clock,
  AlertCircle,
  CheckCircle,
  XCircle,
  Activity
} from 'lucide-react';

interface Task {
  id: number;
  orchestration_id: number;
  name: string;
  description: string;
  goal: string;
  status: TaskStatus;
  agent_config: AgentConfig;
  depends_on: number[];
  created_at: string;
  started_at?: string;
  completed_at?: string;
}

interface AgentConfig {
  agent_type: string;
  model: string;
  system_prompt: string;
  max_tokens?: number;
  temperature?: number;
  capabilities: string[];
  sandbox_profile?: string;
}

type TaskStatus = 'pending' | 'ready' | 'running' | 'completed' | 'failed' | 'cancelled' | 'blocked';

interface LogEntry {
  id: string;
  task_id: number;
  level: 'debug' | 'info' | 'warning' | 'error';
  message: string;
  timestamp: string;
}

interface Artifact {
  name: string;
  artifact_type: string;
  path: string;
  description: string;
  metadata: Record<string, any>;
}

interface OrchestrationMetrics {
  total_tasks: number;
  completed_tasks: number;
  failed_tasks: number;
  running_tasks: number;
  average_duration: number;
  success_rate: number;
}

export const SupervisorPanel: React.FC = () => {
  const [orchestrationId, setOrchestrationId] = useState<number | null>(null);
  const [tasks, setTasks] = useState<Task[]>([]);
  const [selectedTask, setSelectedTask] = useState<Task | null>(null);
  const [logs, setLogs] = useState<LogEntry[]>([]);
  const [artifacts, setArtifacts] = useState<Artifact[]>([]);
  const [metrics, setMetrics] = useState<OrchestrationMetrics | null>(null);
  const [isRunning, setIsRunning] = useState(false);
  const [isPaused, setIsPaused] = useState(false);
  const [searchQuery, setSearchQuery] = useState('');
  const [statusFilter, setStatusFilter] = useState<TaskStatus | 'all'>('all');
  const [expandedTasks, setExpandedTasks] = useState<Set<number>>(new Set());
  const logsEndRef = useRef<HTMLDivElement>(null);
  const [autoScrollLogs, setAutoScrollLogs] = useState(true);

  // Initialize orchestration kernel on mount
  useEffect(() => {
    const initKernel = async () => {
      try {
        await invoke('init_orchestration_kernel');
      } catch (error) {
        console.error('Failed to initialize orchestration kernel:', error);
      }
    };
    initKernel();
  }, []);

  // Set up event listeners
  useEffect(() => {
    const unlistenTaskSelected = listen<number>('task-selected', (event) => {
      const task = tasks.find(t => t.id === event.payload);
      if (task) {
        setSelectedTask(task);
      }
    });

    const unlistenTaskUpdate = listen<Task>('task-updated', (event) => {
      setTasks(prev => prev.map(t => t.id === event.payload.id ? event.payload : t));
    });

    const unlistenLogMessage = listen<LogEntry>('task-log', (event) => {
      setLogs(prev => [...prev, event.payload]);
    });

    return () => {
      unlistenTaskSelected.then(fn => fn());
      unlistenTaskUpdate.then(fn => fn());
      unlistenLogMessage.then(fn => fn());
    };
  }, [tasks]);

  // Auto-scroll logs
  useEffect(() => {
    if (autoScrollLogs && logsEndRef.current) {
      logsEndRef.current.scrollIntoView({ behavior: 'smooth' });
    }
  }, [logs, autoScrollLogs]);

  // Load orchestration data
  const loadOrchestration = useCallback(async (id: number) => {
    try {
      setOrchestrationId(id);
      const tasksData = await invoke<Task[]>('get_orchestration_tasks', { orchestrationId: id });
      setTasks(tasksData);
      
      // Calculate metrics
      const completed = tasksData.filter(t => t.status === 'completed').length;
      const failed = tasksData.filter(t => t.status === 'failed').length;
      const running = tasksData.filter(t => t.status === 'running').length;
      
      setMetrics({
        total_tasks: tasksData.length,
        completed_tasks: completed,
        failed_tasks: failed,
        running_tasks: running,
        average_duration: 0, // TODO: Calculate from timestamps
        success_rate: tasksData.length > 0 ? (completed / tasksData.length) * 100 : 0
      });
    } catch (error) {
      console.error('Failed to load orchestration:', error);
    }
  }, []);

  // Control functions
  const handleStart = async () => {
    if (!orchestrationId) return;
    try {
      await invoke('schedule_orchestration_tasks');
      setIsRunning(true);
      setIsPaused(false);
    } catch (error) {
      console.error('Failed to start orchestration:', error);
    }
  };

  const handlePause = () => {
    setIsPaused(true);
    // TODO: Implement pause functionality
  };

  const handleStop = async () => {
    if (!orchestrationId) return;
    try {
      await invoke('cancel_orchestration', { orchestrationId });
      setIsRunning(false);
      setIsPaused(false);
    } catch (error) {
      console.error('Failed to stop orchestration:', error);
    }
  };

  const handleRetryTask = async (taskId: number) => {
    try {
      await invoke('update_task_status', { taskId, status: 'ready' });
      if (isRunning) {
        await invoke('schedule_orchestration_tasks');
      }
    } catch (error) {
      console.error('Failed to retry task:', error);
    }
  };

  // Filter tasks
  const filteredTasks = tasks.filter(task => {
    const matchesSearch = searchQuery === '' || 
      task.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
      task.description.toLowerCase().includes(searchQuery.toLowerCase());
    
    const matchesStatus = statusFilter === 'all' || task.status === statusFilter;
    
    return matchesSearch && matchesStatus;
  });

  // Toggle task expansion
  const toggleTaskExpansion = (taskId: number) => {
    setExpandedTasks(prev => {
      const next = new Set(prev);
      if (next.has(taskId)) {
        next.delete(taskId);
      } else {
        next.add(taskId);
      }
      return next;
    });
  };

  // Get status color
  const getStatusColor = (status: TaskStatus) => {
    switch (status) {
      case 'completed': return 'text-green-600';
      case 'failed': return 'text-red-600';
      case 'running': return 'text-blue-600';
      case 'blocked': return 'text-orange-600';
      case 'cancelled': return 'text-gray-600';
      default: return 'text-gray-400';
    }
  };

  // Get status icon
  const getStatusIcon = (status: TaskStatus) => {
    switch (status) {
      case 'completed': return <CheckCircle className="w-4 h-4" />;
      case 'failed': return <XCircle className="w-4 h-4" />;
      case 'running': return <Activity className="w-4 h-4 animate-pulse" />;
      case 'blocked': return <AlertCircle className="w-4 h-4" />;
      default: return <Clock className="w-4 h-4" />;
    }
  };

  // Get log level color
  const getLogLevelColor = (level: LogEntry['level']) => {
    switch (level) {
      case 'error': return 'text-red-600';
      case 'warning': return 'text-orange-600';
      case 'info': return 'text-blue-600';
      default: return 'text-gray-600';
    }
  };

  return (
    <div className="h-full flex flex-col space-y-4 p-4">
      {/* Control Bar */}
      <Card>
        <CardContent className="flex items-center justify-between p-4">
          <div className="flex items-center space-x-2">
            <Button
              onClick={handleStart}
              disabled={isRunning && !isPaused}
              variant="default"
              size="sm"
            >
              <Play className="w-4 h-4 mr-1" />
              Start
            </Button>
            <Button
              onClick={handlePause}
              disabled={!isRunning || isPaused}
              variant="outline"
              size="sm"
            >
              <Pause className="w-4 h-4 mr-1" />
              Pause
            </Button>
            <Button
              onClick={handleStop}
              disabled={!isRunning}
              variant="destructive"
              size="sm"
            >
              <Square className="w-4 h-4 mr-1" />
              Stop
            </Button>
          </div>
          
          <div className="flex items-center space-x-4">
            <div className="flex items-center space-x-2">
              <Search className="w-4 h-4 text-gray-400" />
              <Input
                type="text"
                placeholder="Search tasks..."
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
                className="w-64"
              />
            </div>
            
            <Select value={statusFilter} onValueChange={(value) => setStatusFilter(value as TaskStatus | 'all')}>
              <SelectTrigger className="w-32">
                <Filter className="w-4 h-4 mr-2" />
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
        </CardContent>
      </Card>

      {/* Main Content */}
      <div className="flex-1 grid grid-cols-3 gap-4 min-h-0">
        {/* Task List */}
        <Card className="col-span-1 flex flex-col">
          <CardHeader>
            <CardTitle>Tasks</CardTitle>
          </CardHeader>
          <CardContent className="flex-1 overflow-auto">
            <div className="space-y-2">
              {filteredTasks.map(task => (
                <div
                  key={task.id}
                  className={`border rounded-lg p-3 cursor-pointer transition-colors ${
                    selectedTask?.id === task.id ? 'bg-blue-50 border-blue-300' : 'hover:bg-gray-50'
                  }`}
                  onClick={() => setSelectedTask(task)}
                >
                  <div className="flex items-center justify-between">
                    <div className="flex items-center space-x-2">
                      <button
                        onClick={(e) => {
                          e.stopPropagation();
                          toggleTaskExpansion(task.id);
                        }}
                        className="p-0.5"
                      >
                        {expandedTasks.has(task.id) ? 
                          <ChevronDown className="w-4 h-4" /> : 
                          <ChevronRight className="w-4 h-4" />
                        }
                      </button>
                      <span className={getStatusColor(task.status)}>
                        {getStatusIcon(task.status)}
                      </span>
                      <span className="font-medium">{task.name}</span>
                    </div>
                    <div className="flex items-center space-x-2">
                      <Badge variant="outline" className="text-xs">
                        {task.agent_config.agent_type}
                      </Badge>
                      {task.status === 'failed' && (
                        <Button
                          size="sm"
                          variant="ghost"
                          onClick={(e) => {
                            e.stopPropagation();
                            handleRetryTask(task.id);
                          }}
                        >
                          <RotateCcw className="w-3 h-3" />
                        </Button>
                      )}
                    </div>
                  </div>
                  
                  {expandedTasks.has(task.id) && (
                    <div className="mt-2 text-sm text-gray-600">
                      <p>{task.description}</p>
                      {task.depends_on.length > 0 && (
                        <p className="mt-1">
                          Dependencies: {task.depends_on.join(', ')}
                        </p>
                      )}
                    </div>
                  )}
                </div>
              ))}
            </div>
          </CardContent>
        </Card>

        {/* Details Panel */}
        <Card className="col-span-2 flex flex-col">
          <CardHeader>
            <CardTitle>Details</CardTitle>
          </CardHeader>
          <CardContent className="flex-1 overflow-auto">
            {selectedTask ? (
              <Tabs defaultValue="logs" className="h-full flex flex-col">
                <TabsList>
                  <TabsTrigger value="logs">Logs</TabsTrigger>
                  <TabsTrigger value="artifacts">Artifacts</TabsTrigger>
                  <TabsTrigger value="metrics">Metrics</TabsTrigger>
                  <TabsTrigger value="details">Task Details</TabsTrigger>
                </TabsList>
                
                <TabsContent value="logs" className="flex-1 overflow-auto">
                  <div className="space-y-1">
                    <div className="flex justify-between items-center mb-2">
                      <h3 className="font-medium">Real-time Logs</h3>
                      <label className="flex items-center space-x-2">
                        <input
                          type="checkbox"
                          checked={autoScrollLogs}
                          onChange={(e) => setAutoScrollLogs(e.target.checked)}
                        />
                        <span className="text-sm">Auto-scroll</span>
                      </label>
                    </div>
                    {logs
                      .filter(log => log.task_id === selectedTask.id)
                      .map(log => (
                        <div key={log.id} className="font-mono text-sm">
                          <span className="text-gray-500">
                            {new Date(log.timestamp).toLocaleTimeString()}
                          </span>
                          <span className={`ml-2 ${getLogLevelColor(log.level)}`}>
                            [{log.level.toUpperCase()}]
                          </span>
                          <span className="ml-2">{log.message}</span>
                        </div>
                      ))}
                    <div ref={logsEndRef} />
                  </div>
                </TabsContent>
                
                <TabsContent value="artifacts" className="flex-1 overflow-auto">
                  <div className="space-y-2">
                    {artifacts
                      .filter(a => a.path.includes(`task_${selectedTask.id}`))
                      .map((artifact, idx) => (
                        <div key={idx} className="border rounded-lg p-3">
                          <div className="flex items-center space-x-2">
                            <FileText className="w-4 h-4 text-gray-400" />
                            <span className="font-medium">{artifact.name}</span>
                            <Badge variant="outline" className="text-xs">
                              {artifact.artifact_type}
                            </Badge>
                          </div>
                          <p className="text-sm text-gray-600 mt-1">
                            {artifact.description}
                          </p>
                          <p className="text-xs text-gray-400 mt-1">
                            {artifact.path}
                          </p>
                        </div>
                      ))}
                  </div>
                </TabsContent>
                
                <TabsContent value="metrics" className="flex-1">
                  {metrics && (
                    <div className="grid grid-cols-2 gap-4">
                      <Card>
                        <CardHeader className="pb-2">
                          <CardTitle className="text-sm">Total Tasks</CardTitle>
                        </CardHeader>
                        <CardContent>
                          <p className="text-2xl font-bold">{metrics.total_tasks}</p>
                        </CardContent>
                      </Card>
                      
                      <Card>
                        <CardHeader className="pb-2">
                          <CardTitle className="text-sm">Success Rate</CardTitle>
                        </CardHeader>
                        <CardContent>
                          <p className="text-2xl font-bold">
                            {metrics.success_rate.toFixed(1)}%
                          </p>
                        </CardContent>
                      </Card>
                      
                      <Card>
                        <CardHeader className="pb-2">
                          <CardTitle className="text-sm">Running</CardTitle>
                        </CardHeader>
                        <CardContent>
                          <p className="text-2xl font-bold text-blue-600">
                            {metrics.running_tasks}
                          </p>
                        </CardContent>
                      </Card>
                      
                      <Card>
                        <CardHeader className="pb-2">
                          <CardTitle className="text-sm">Failed</CardTitle>
                        </CardHeader>
                        <CardContent>
                          <p className="text-2xl font-bold text-red-600">
                            {metrics.failed_tasks}
                          </p>
                        </CardContent>
                      </Card>
                    </div>
                  )}
                </TabsContent>
                
                <TabsContent value="details" className="flex-1 overflow-auto">
                  <div className="space-y-4">
                    <div>
                      <h3 className="font-medium mb-2">Task Information</h3>
                      <dl className="space-y-2">
                        <div>
                          <dt className="text-sm text-gray-500">ID</dt>
                          <dd className="font-mono">{selectedTask.id}</dd>
                        </div>
                        <div>
                          <dt className="text-sm text-gray-500">Name</dt>
                          <dd>{selectedTask.name}</dd>
                        </div>
                        <div>
                          <dt className="text-sm text-gray-500">Goal</dt>
                          <dd>{selectedTask.goal}</dd>
                        </div>
                        <div>
                          <dt className="text-sm text-gray-500">Status</dt>
                          <dd className="flex items-center space-x-2">
                            <span className={getStatusColor(selectedTask.status)}>
                              {getStatusIcon(selectedTask.status)}
                            </span>
                            <span>{selectedTask.status}</span>
                          </dd>
                        </div>
                      </dl>
                    </div>
                    
                    <div>
                      <h3 className="font-medium mb-2">Agent Configuration</h3>
                      <dl className="space-y-2">
                        <div>
                          <dt className="text-sm text-gray-500">Type</dt>
                          <dd>{selectedTask.agent_config.agent_type}</dd>
                        </div>
                        <div>
                          <dt className="text-sm text-gray-500">Model</dt>
                          <dd>{selectedTask.agent_config.model}</dd>
                        </div>
                        <div>
                          <dt className="text-sm text-gray-500">Temperature</dt>
                          <dd>{selectedTask.agent_config.temperature || 0.7}</dd>
                        </div>
                        <div>
                          <dt className="text-sm text-gray-500">Max Tokens</dt>
                          <dd>{selectedTask.agent_config.max_tokens || 4096}</dd>
                        </div>
                      </dl>
                    </div>
                    
                    {selectedTask.depends_on.length > 0 && (
                      <div>
                        <h3 className="font-medium mb-2">Dependencies</h3>
                        <ul className="space-y-1">
                          {selectedTask.depends_on.map(dep => {
                            const depTask = tasks.find(t => t.id === dep);
                            return (
                              <li key={dep} className="flex items-center space-x-2">
                                <span className={depTask ? getStatusColor(depTask.status) : ''}>
                                  {depTask ? getStatusIcon(depTask.status) : null}
                                </span>
                                <span>
                                  Task #{dep} {depTask ? `- ${depTask.name}` : ''}
                                </span>
                              </li>
                            );
                          })}
                        </ul>
                      </div>
                    )}
                  </div>
                </TabsContent>
              </Tabs>
            ) : (
              <div className="flex items-center justify-center h-full text-gray-400">
                Select a task to view details
              </div>
            )}
          </CardContent>
        </Card>
      </div>
    </div>
  );
};