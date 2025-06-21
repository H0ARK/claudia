import React, { useState } from 'react';
import { SupervisorPanel } from './SupervisorPanel';
import { Button } from '../ui/button';
import { invoke } from '@tauri-apps/api/core';

export const SupervisorPanelExample: React.FC = () => {
  const [orchestrationId, setOrchestrationId] = useState<number | null>(null);
  const [isCreating, setIsCreating] = useState(false);

  const createExampleOrchestration = async () => {
    setIsCreating(true);
    try {
      // Initialize kernel if not already done
      await invoke('init_orchestration_kernel');
      
      // Create a new orchestration
      const id = await invoke<number>('create_orchestration', {
        rootGoal: 'Example multi-agent orchestration workflow'
      });
      
      // Add some example tasks
      await invoke('add_orchestration_task', {
        orchestrationId: id,
        name: 'Plan Project',
        description: 'Create a detailed project plan',
        goal: 'Analyze requirements and create a comprehensive project plan',
        agentType: 'planner',
        model: 'claude-3-sonnet',
        dependsOn: []
      });
      
      const task2Id = await invoke<number>('add_orchestration_task', {
        orchestrationId: id,
        name: 'Generate Code',
        description: 'Generate initial code structure',
        goal: 'Create the basic code structure based on the project plan',
        agentType: 'code_generator',
        model: 'claude-3-sonnet',
        dependsOn: []
      });
      
      await invoke('add_orchestration_task', {
        orchestrationId: id,
        name: 'Write Tests',
        description: 'Create unit tests for the generated code',
        goal: 'Write comprehensive unit tests for all code modules',
        agentType: 'test_writer',
        model: 'claude-3-sonnet',
        dependsOn: [task2Id]
      });
      
      await invoke('add_orchestration_task', {
        orchestrationId: id,
        name: 'Generate Documentation',
        description: 'Create project documentation',
        goal: 'Generate comprehensive documentation for the project',
        agentType: 'doc_writer',
        model: 'claude-3-sonnet',
        dependsOn: [task2Id]
      });
      
      setOrchestrationId(id);
    } catch (error) {
      console.error('Failed to create example orchestration:', error);
    } finally {
      setIsCreating(false);
    }
  };

  return (
    <div className="h-screen flex flex-col">
      {!orchestrationId ? (
        <div className="flex items-center justify-center h-full">
          <div className="text-center space-y-4">
            <h2 className="text-2xl font-bold">Orchestration Supervisor Demo</h2>
            <p className="text-gray-600">
              Create an example orchestration to see the supervisor panel in action
            </p>
            <Button
              onClick={createExampleOrchestration}
              disabled={isCreating}
              size="lg"
            >
              {isCreating ? 'Creating...' : 'Create Example Orchestration'}
            </Button>
          </div>
        </div>
      ) : (
        <SupervisorPanel />
      )}
    </div>
  );
};