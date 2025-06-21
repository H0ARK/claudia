import React, { useState, useEffect } from 'react';
import { DagVisualizer } from './DagVisualizer';
import { Card } from '../ui/card';
import { Button } from '../ui/button';
import { Input } from '../ui/input';
import { Textarea } from '../ui/textarea';
import { Badge } from '../ui/badge';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '../ui/tabs';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { Plus, Play, Pause, RotateCcw, Trash2 } from 'lucide-react';

interface TaskDetails {
  id: number;
  name: string;
  description: string;
  goal: string;
  status: string;
  output?: string;
  error?: string;
  createdAt: string;
  startedAt?: string;
  completedAt?: string;
}

export const DagVisualizerExample: React.FC = () => {
  const [orchestrationId, setOrchestrationId] = useState<number | null>(null);
  const [rootGoal, setRootGoal] = useState('');
  const [selectedTask, setSelectedTask] = useState<TaskDetails | null>(null);
  const [isCreating, setIsCreating] = useState(false);
  const [orchestrationStatus, setOrchestrationStatus] = useState<any>(null);

  // Initialize orchestration kernel on mount
  useEffect(() => {
    invoke('init_orchestration_kernel').catch(console.error);
  }, []);

  // Listen for task selection events
  useEffect(() => {
    const unsubscribe = listen<number>('task-selected', (event) => {
      loadTaskDetails(event.payload);
    });

    return () => {
      unsubscribe.then(fn => fn());
    };
  }, []);

  // Load orchestration status periodically
  useEffect(() => {
    if (!orchestrationId) return;

    const loadStatus = async () => {
      try {
        const status = await invoke('get_orchestration_status', { orchestrationId });
        setOrchestrationStatus(status);
      } catch (error) {
        console.error('Failed to load orchestration status:', error);
      }
    };

    loadStatus();
    const interval = setInterval(loadStatus, 2000);

    return () => clearInterval(interval);
  }, [orchestrationId]);

  const createOrchestration = async () => {
    if (!rootGoal.trim()) return;

    setIsCreating(true);
    try {
      const id = await invoke<number>('create_orchestration', { rootGoal });
      setOrchestrationId(id);
      setRootGoal('');
      
      // Schedule initial tasks
      await invoke('schedule_orchestration_tasks');
    } catch (error) {
      console.error('Failed to create orchestration:', error);
    } finally {
      setIsCreating(false);
    }
  };

  const loadTaskDetails = async (taskId: number) => {
    try {
      // In a real implementation, you'd have a get_task command
      // For now, we'll use a placeholder
      setSelectedTask({
        id: taskId,
        name: `Task ${taskId}`,
        description: 'Task description',
        goal: 'Task goal',
        status: 'pending',
        createdAt: new Date().toISOString(),
      });
    } catch (error) {
      console.error('Failed to load task details:', error);
    }
  };

  const scheduleNextBatch = async () => {
    try {
      await invoke('schedule_orchestration_tasks');
    } catch (error) {
      console.error('Failed to schedule tasks:', error);
    }
  };

  const cancelOrchestration = async () => {
    if (!orchestrationId) return;
    
    try {
      await invoke('cancel_orchestration', { orchestrationId });
    } catch (error) {
      console.error('Failed to cancel orchestration:', error);
    }
  };

  return (
    <div className="h-full flex flex-col gap-4 p-4">
      <Card className="p-4">
        <h2 className="text-xl font-semibold mb-4">Orchestration DAG Visualizer</h2>
        
        {!orchestrationId ? (
          <div className="space-y-4">
            <div>
              <label className="block text-sm font-medium mb-2">Root Goal</label>
              <Textarea
                value={rootGoal}
                onChange={(e) => setRootGoal(e.target.value)}
                placeholder="Enter the main goal for the orchestration..."
                className="min-h-[100px]"
              />
            </div>
            <Button
              onClick={createOrchestration}
              disabled={!rootGoal.trim() || isCreating}
              className="w-full"
            >
              <Plus className="w-4 h-4 mr-2" />
              Create Orchestration
            </Button>
          </div>
        ) : (
          <div className="space-y-4">
            <div className="flex items-center justify-between">
              <div>
                <h3 className="font-medium">Orchestration #{orchestrationId}</h3>
                {orchestrationStatus && (
                  <div className="flex items-center gap-4 mt-2 text-sm text-gray-600">
                    <span>Status: <Badge>{orchestrationStatus.status}</Badge></span>
                    <span>Progress: {orchestrationStatus.progress?.toFixed(1)}%</span>
                    <span>Tasks: {orchestrationStatus.completed_tasks}/{orchestrationStatus.total_tasks}</span>
                  </div>
                )}
              </div>
              <div className="flex gap-2">
                <Button onClick={scheduleNextBatch} size="sm" variant="outline">
                  <Play className="w-4 h-4 mr-1" />
                  Schedule
                </Button>
                <Button onClick={cancelOrchestration} size="sm" variant="outline">
                  <Pause className="w-4 h-4 mr-1" />
                  Cancel
                </Button>
                <Button
                  onClick={() => {
                    setOrchestrationId(null);
                    setOrchestrationStatus(null);
                    setSelectedTask(null);
                  }}
                  size="sm"
                  variant="outline"
                >
                  <RotateCcw className="w-4 h-4 mr-1" />
                  New
                </Button>
              </div>
            </div>
          </div>
        )}
      </Card>

      {orchestrationId && (
        <div className="flex-1 flex gap-4">
          <Card className="flex-1 overflow-hidden">
            <DagVisualizer orchestrationId={orchestrationId} className="h-full" />
          </Card>

          {selectedTask && (
            <Card className="w-96 p-4 overflow-y-auto">
              <h3 className="font-semibold mb-4">Task Details</h3>
              <Tabs defaultValue="info">
                <TabsList className="grid w-full grid-cols-2">
                  <TabsTrigger value="info">Info</TabsTrigger>
                  <TabsTrigger value="output">Output</TabsTrigger>
                </TabsList>
                
                <TabsContent value="info" className="space-y-3">
                  <div>
                    <label className="text-sm font-medium text-gray-500">ID</label>
                    <p className="text-sm">#{selectedTask.id}</p>
                  </div>
                  
                  <div>
                    <label className="text-sm font-medium text-gray-500">Name</label>
                    <p className="text-sm">{selectedTask.name}</p>
                  </div>
                  
                  <div>
                    <label className="text-sm font-medium text-gray-500">Description</label>
                    <p className="text-sm">{selectedTask.description}</p>
                  </div>
                  
                  <div>
                    <label className="text-sm font-medium text-gray-500">Goal</label>
                    <p className="text-sm">{selectedTask.goal}</p>
                  </div>
                  
                  <div>
                    <label className="text-sm font-medium text-gray-500">Status</label>
                    <Badge variant="outline">{selectedTask.status}</Badge>
                  </div>
                  
                  <div>
                    <label className="text-sm font-medium text-gray-500">Created</label>
                    <p className="text-sm">{new Date(selectedTask.createdAt).toLocaleString()}</p>
                  </div>
                  
                  {selectedTask.startedAt && (
                    <div>
                      <label className="text-sm font-medium text-gray-500">Started</label>
                      <p className="text-sm">{new Date(selectedTask.startedAt).toLocaleString()}</p>
                    </div>
                  )}
                  
                  {selectedTask.completedAt && (
                    <div>
                      <label className="text-sm font-medium text-gray-500">Completed</label>
                      <p className="text-sm">{new Date(selectedTask.completedAt).toLocaleString()}</p>
                    </div>
                  )}
                </TabsContent>
                
                <TabsContent value="output" className="space-y-3">
                  {selectedTask.output ? (
                    <pre className="text-xs bg-gray-50 p-3 rounded overflow-x-auto">
                      {selectedTask.output}
                    </pre>
                  ) : selectedTask.error ? (
                    <pre className="text-xs bg-red-50 text-red-700 p-3 rounded overflow-x-auto">
                      {selectedTask.error}
                    </pre>
                  ) : (
                    <p className="text-sm text-gray-500 italic">No output yet</p>
                  )}
                </TabsContent>
              </Tabs>
            </Card>
          )}
        </div>
      )}
    </div>
  );
};