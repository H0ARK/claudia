import React, { useState } from 'react';
import { OrchestrationLiveView } from './OrchestrationLiveView';
import { orchestrationClient } from '../../lib/orchestration';
import { useOrchestration } from '../../lib/useOrchestration';
import { Button } from '../ui/button';
import { Input } from '../ui/input';
import { Card } from '../ui/card';

/**
 * Example component demonstrating the OrchestrationLiveView with the useOrchestration hook
 */
export const OrchestrationLiveViewExample: React.FC = () => {
  const [orchestrationId, setOrchestrationId] = useState<number | null>(null);
  const [rootGoal, setRootGoal] = useState('');
  const [selectedTaskId, setSelectedTaskId] = useState<number | null>(null);
  const [isInitialized, setIsInitialized] = useState(false);

  // Use the orchestration hook if we have an orchestration ID
  const orchestrationData = useOrchestration(orchestrationId || 0, {
    autoSubscribe: !!orchestrationId,
    maxEventHistory: 50,
  });

  const handleInitialize = async () => {
    try {
      await orchestrationClient.init();
      setIsInitialized(true);
    } catch (error) {
      console.error('Failed to initialize orchestration kernel:', error);
    }
  };

  const handleCreateOrchestration = async () => {
    if (!rootGoal.trim()) return;

    try {
      const id = await orchestrationClient.createOrchestration(rootGoal);
      setOrchestrationId(id);
      setRootGoal('');
      
      // Schedule initial tasks
      await orchestrationClient.scheduleTasks();
    } catch (error) {
      console.error('Failed to create orchestration:', error);
    }
  };

  const handleAddTask = async () => {
    if (!orchestrationId) return;

    try {
      await orchestrationClient.addTask({
        orchestrationId,
        name: 'Example Task',
        description: 'An example task added dynamically',
        goal: 'Complete the example task',
        agentType: 'worker',
        model: 'claude-3-5-sonnet-20241022',
        dependsOn: [],
      });
      
      // Refresh to get the new task
      orchestrationData.refresh();
    } catch (error) {
      console.error('Failed to add task:', error);
    }
  };

  return (
    <div className="space-y-6">
      <Card className="p-6">
        <h1 className="text-2xl font-bold mb-4">Orchestration Live View Example</h1>
        
        {!isInitialized ? (
          <div className="space-y-4">
            <p className="text-muted-foreground">
              Initialize the orchestration kernel to get started.
            </p>
            <Button onClick={handleInitialize}>Initialize Orchestration Kernel</Button>
          </div>
        ) : !orchestrationId ? (
          <div className="space-y-4">
            <p className="text-muted-foreground">
              Create a new orchestration to see live updates.
            </p>
            <div className="flex gap-2">
              <Input
                placeholder="Enter root goal..."
                value={rootGoal}
                onChange={(e) => setRootGoal(e.target.value)}
                onKeyPress={(e) => e.key === 'Enter' && handleCreateOrchestration()}
              />
              <Button onClick={handleCreateOrchestration}>Create Orchestration</Button>
            </div>
          </div>
        ) : (
          <div className="space-y-4">
            <div className="flex items-center justify-between">
              <div>
                <h2 className="text-lg font-semibold">Orchestration #{orchestrationId}</h2>
                <p className="text-sm text-muted-foreground">
                  {orchestrationData.isConnected ? '🟢 Connected' : '🔴 Disconnected'}
                  {' • '}
                  {orchestrationData.events.length} events received
                </p>
              </div>
              <div className="flex gap-2">
                <Button variant="outline" size="sm" onClick={handleAddTask}>
                  Add Task
                </Button>
                <Button variant="outline" size="sm" onClick={orchestrationData.refresh}>
                  Refresh
                </Button>
                <Button 
                  variant="outline" 
                  size="sm" 
                  onClick={() => orchestrationClient.scheduleTasks()}
                >
                  Schedule Tasks
                </Button>
              </div>
            </div>
          </div>
        )}
      </Card>

      {orchestrationId && (
        <>
          <OrchestrationLiveView
            orchestrationId={orchestrationId}
            onTaskSelect={setSelectedTaskId}
          />
          
          {selectedTaskId && (
            <Card className="p-4">
              <h3 className="text-lg font-semibold mb-2">Selected Task Details</h3>
              <div className="space-y-2">
                <p>Task ID: {selectedTaskId}</p>
                {orchestrationData.tasks
                  .filter(task => task.id === selectedTaskId)
                  .map(task => (
                    <div key={task.id} className="space-y-1">
                      <p><strong>Name:</strong> {task.name}</p>
                      <p><strong>Status:</strong> {task.status}</p>
                      <p><strong>Goal:</strong> {task.goal}</p>
                      {task.output && (
                        <div>
                          <strong>Output:</strong>
                          <pre className="mt-1 p-2 bg-secondary rounded text-xs">
                            {task.output}
                          </pre>
                        </div>
                      )}
                      {task.error && (
                        <div>
                          <strong>Error:</strong>
                          <pre className="mt-1 p-2 bg-destructive/10 rounded text-xs text-destructive">
                            {task.error}
                          </pre>
                        </div>
                      )}
                    </div>
                  ))}
              </div>
            </Card>
          )}
          
          {/* Recent Events from Hook */}
          <Card className="p-4">
            <h3 className="text-lg font-semibold mb-2">Recent Events (from Hook)</h3>
            <div className="max-h-48 overflow-y-auto space-y-1">
              {orchestrationData.events.slice(0, 10).map((event, index) => (
                <div key={`${event.type}-${event.timestamp}-${index}`} className="text-xs font-mono">
                  <span className="text-muted-foreground">
                    {new Date(event.timestamp).toLocaleTimeString()}
                  </span>
                  {' '}
                  <span className="font-semibold">{event.type}</span>
                  {' '}
                  {'task_id' in event && <span>Task #{event.task_id}</span>}
                </div>
              ))}
            </div>
          </Card>
        </>
      )}
    </div>
  );
};