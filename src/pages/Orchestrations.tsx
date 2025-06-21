import React, { useState, useEffect } from 'react';
import { Plus, Play, Pause, Stop, RefreshCw } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from '@/components/ui/dialog';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Textarea } from '@/components/ui/textarea';
import { Badge } from '@/components/ui/badge';
import { orchestrationClient } from '@/lib/orchestration';
import { DagVisualizer } from '@/components/orchestration/DagVisualizer';
import { SupervisorPanel } from '@/components/orchestration/SupervisorPanel';
import { useAllOrchestrations } from '@/lib/useOrchestration';

interface CreateOrchestrationDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onCreated: (id: number) => void;
}

function CreateOrchestrationDialog({ open, onOpenChange, onCreated }: CreateOrchestrationDialogProps) {
  const [name, setName] = useState('');
  const [goal, setGoal] = useState('');
  const [creating, setCreating] = useState(false);

  const handleCreate = async () => {
    setCreating(true);
    try {
      const id = await orchestrationClient.createOrchestration(goal, { name });
      onCreated(id);
      onOpenChange(false);
      setName('');
      setGoal('');
    } catch (error) {
      console.error('Failed to create orchestration:', error);
    } finally {
      setCreating(false);
    }
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-[425px]">
        <DialogHeader>
          <DialogTitle>Create New Orchestration</DialogTitle>
          <DialogDescription>
            Define a high-level goal and let the planner agent decompose it into tasks.
          </DialogDescription>
        </DialogHeader>
        <div className="grid gap-4 py-4">
          <div className="grid grid-cols-4 items-center gap-4">
            <Label htmlFor="name" className="text-right">
              Name
            </Label>
            <Input
              id="name"
              value={name}
              onChange={(e) => setName(e.target.value)}
              className="col-span-3"
              placeholder="e.g., Build Landing Page"
            />
          </div>
          <div className="grid grid-cols-4 items-start gap-4">
            <Label htmlFor="goal" className="text-right pt-2">
              Goal
            </Label>
            <Textarea
              id="goal"
              value={goal}
              onChange={(e) => setGoal(e.target.value)}
              className="col-span-3"
              placeholder="e.g., Create a responsive landing page with hero section, features, and contact form"
              rows={4}
            />
          </div>
        </div>
        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)}>
            Cancel
          </Button>
          <Button onClick={handleCreate} disabled={!goal || creating}>
            {creating ? 'Creating...' : 'Create'}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

function OrchestrationCard({ orchestration, onClick }: any) {
  const getStatusColor = (status: string) => {
    switch (status) {
      case 'planning': return 'blue';
      case 'running': return 'yellow';
      case 'completed': return 'green';
      case 'failed': return 'red';
      case 'cancelled': return 'gray';
      default: return 'gray';
    }
  };

  const getStatusIcon = (status: string) => {
    switch (status) {
      case 'planning': return '📋';
      case 'running': return '⚡';
      case 'completed': return '✅';
      case 'failed': return '❌';
      case 'cancelled': return '⏹️';
      default: return '⏸️';
    }
  };

  return (
    <Card className="cursor-pointer hover:shadow-lg transition-shadow" onClick={onClick}>
      <CardHeader>
        <div className="flex items-center justify-between">
          <CardTitle className="text-lg">{orchestration.name || `Orchestration #${orchestration.id}`}</CardTitle>
          <Badge variant={getStatusColor(orchestration.status) as any}>
            {getStatusIcon(orchestration.status)} {orchestration.status}
          </Badge>
        </div>
        <CardDescription>{orchestration.root_goal}</CardDescription>
      </CardHeader>
      <CardContent>
        <div className="grid grid-cols-2 gap-2 text-sm">
          <div>
            <span className="text-muted-foreground">Tasks:</span>{' '}
            <span className="font-medium">{orchestration.total_tasks}</span>
          </div>
          <div>
            <span className="text-muted-foreground">Completed:</span>{' '}
            <span className="font-medium">{orchestration.completed_tasks}</span>
          </div>
          <div className="col-span-2">
            <span className="text-muted-foreground">Progress:</span>{' '}
            <div className="w-full bg-gray-200 rounded-full h-2 mt-1">
              <div
                className="bg-blue-600 h-2 rounded-full"
                style={{ width: `${orchestration.progress || 0}%` }}
              />
            </div>
          </div>
        </div>
      </CardContent>
    </Card>
  );
}

export default function Orchestrations() {
  const [createDialogOpen, setCreateDialogOpen] = useState(false);
  const [selectedOrchestration, setSelectedOrchestration] = useState<number | null>(null);
  const { orchestrations, isConnected } = useAllOrchestrations();

  useEffect(() => {
    orchestrationClient.init().catch(console.error);
  }, []);

  const handleOrchestrationCreated = (id: number) => {
    setSelectedOrchestration(id);
  };

  if (selectedOrchestration !== null) {
    return (
      <div className="h-full flex flex-col">
        <div className="flex items-center justify-between p-4 border-b">
          <Button
            variant="ghost"
            onClick={() => setSelectedOrchestration(null)}
          >
            ← Back to Orchestrations
          </Button>
          <div className="flex items-center gap-2">
            <span className={`w-2 h-2 rounded-full ${isConnected ? 'bg-green-500' : 'bg-red-500'}`} />
            <span className="text-sm text-muted-foreground">
              {isConnected ? 'Connected' : 'Disconnected'}
            </span>
          </div>
        </div>
        <div className="flex-1 flex">
          <div className="flex-1">
            <DagVisualizer orchestrationId={selectedOrchestration} />
          </div>
          <div className="w-1/3 border-l">
            <SupervisorPanel orchestrationId={selectedOrchestration} />
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className="container mx-auto p-6">
      <div className="flex items-center justify-between mb-6">
        <div>
          <h1 className="text-3xl font-bold">Orchestrations</h1>
          <p className="text-muted-foreground mt-1">
            Create and manage multi-agent workflows
          </p>
        </div>
        <Button onClick={() => setCreateDialogOpen(true)}>
          <Plus className="w-4 h-4 mr-2" />
          New Orchestration
        </Button>
      </div>

      {orchestrations.length === 0 ? (
        <Card className="p-12">
          <div className="text-center">
            <div className="text-6xl mb-4">🎭</div>
            <h2 className="text-2xl font-semibold mb-2">No Orchestrations Yet</h2>
            <p className="text-muted-foreground mb-6">
              Create your first orchestration to coordinate multiple agents working together
            </p>
            <Button onClick={() => setCreateDialogOpen(true)}>
              <Plus className="w-4 h-4 mr-2" />
              Create Your First Orchestration
            </Button>
          </div>
        </Card>
      ) : (
        <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
          {orchestrations.map((orchestration) => (
            <OrchestrationCard
              key={orchestration.id}
              orchestration={orchestration}
              onClick={() => setSelectedOrchestration(orchestration.id)}
            />
          ))}
        </div>
      )}

      <CreateOrchestrationDialog
        open={createDialogOpen}
        onOpenChange={setCreateDialogOpen}
        onCreated={handleOrchestrationCreated}
      />
    </div>
  );
}