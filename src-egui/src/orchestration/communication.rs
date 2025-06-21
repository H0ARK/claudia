use crate::models::{Message, MessageId, AgentId};
use anyhow::Result;
use crossbeam_channel::{bounded, Receiver, Sender};
use dashmap::DashMap;
use std::sync::Arc;

pub struct MessageBus {
    global_channel: (Sender<Message>, Receiver<Message>),
    agent_channels: Arc<DashMap<AgentId, Sender<Message>>>,
    message_history: Arc<DashMap<MessageId, Message>>,
}

impl MessageBus {
    pub fn new() -> Self {
        let (tx, rx) = bounded(1000);
        
        Self {
            global_channel: (tx, rx),
            agent_channels: Arc::new(DashMap::new()),
            message_history: Arc::new(DashMap::new()),
        }
    }

    pub fn register_agent(&self, agent_id: AgentId) -> Receiver<Message> {
        let (tx, rx) = bounded(100);
        self.agent_channels.insert(agent_id, tx);
        rx
    }

    pub fn unregister_agent(&self, agent_id: &AgentId) {
        self.agent_channels.remove(agent_id);
    }

    pub async fn send(&self, message: Message) -> Result<()> {
        // Store in history
        self.message_history.insert(message.id.clone(), message.clone());

        // Route based on target
        match &message.to {
            crate::models::MessageTarget::Agent(agent_id) => {
                if let Some(tx) = self.agent_channels.get(agent_id) {
                    tx.send(message.clone())?;
                }
            }
            crate::models::MessageTarget::Broadcast => {
                for channel in self.agent_channels.iter() {
                    let _ = channel.send(message.clone());
                }
            }
            crate::models::MessageTarget::Group(agents) => {
                for agent_id in agents {
                    if let Some(tx) = self.agent_channels.get(agent_id) {
                        let _ = tx.send(message.clone());
                    }
                }
            }
            crate::models::MessageTarget::Role(_role_name) => {
                // In a real implementation, we'd look up agents by role
                // For now, broadcast to all
                for channel in self.agent_channels.iter() {
                    let _ = channel.send(message.clone());
                }
            }
            crate::models::MessageTarget::Orchestrator => {
                self.global_channel.0.send(message.clone())?;
            }
        }

        Ok(())
    }

    pub async fn receive(&self) -> Result<Message> {
        Ok(self.global_channel.1.recv()?)
    }

    pub fn get_message_history(&self) -> Vec<Message> {
        self.message_history
            .iter()
            .map(|entry| entry.value().clone())
            .collect()
    }

    pub fn get_agent_message_history(&self, agent_id: &AgentId) -> Vec<Message> {
        self.message_history
            .iter()
            .filter(|entry| {
                let msg = entry.value();
                &msg.from == agent_id || matches!(&msg.to, 
                    crate::models::MessageTarget::Agent(id) if id == agent_id
                )
            })
            .map(|entry| entry.value().clone())
            .collect()
    }
}