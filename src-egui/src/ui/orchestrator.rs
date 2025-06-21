use crate::models::{Agent, AgentStatus, Priority};
use crate::orchestration::OrchestrationEngine;
use egui::{Color32, Context, RichText};
use egui_plot::{Bar, BarChart, Plot};

pub struct OrchestratorView {
    selected_agent: Option<usize>,
    message_filter: MessageFilter,
    show_metrics: bool,
}

#[derive(Default)]
struct MessageFilter {
    show_tasks: bool,
    show_status: bool,
    show_results: bool,
    show_errors: bool,
}

impl OrchestratorView {
    pub fn new() -> Self {
        Self {
            selected_agent: None,
            message_filter: MessageFilter {
                show_tasks: true,
                show_status: true,
                show_results: true,
                show_errors: true,
            },
            show_metrics: false,
        }
    }

    pub fn show(&mut self, ctx: &Context, engine: &mut OrchestrationEngine) {
        // Left panel - Agent status
        egui::SidePanel::left("agent_panel")
            .default_width(300.0)
            .show(ctx, |ui| {
                self.show_agent_panel(ui, engine);
            });

        // Right panel - Metrics
        if self.show_metrics {
            egui::SidePanel::right("metrics_panel")
                .default_width(350.0)
                .show(ctx, |ui| {
                    self.show_metrics_panel(ui, engine);
                });
        }

        // Bottom panel - Message log
        egui::TopBottomPanel::bottom("message_panel")
            .default_height(200.0)
            .resizable(true)
            .show(ctx, |ui| {
                self.show_message_log(ui, engine);
            });

        // Central panel - Workflow visualization
        egui::CentralPanel::default().show(ctx, |ui| {
            self.show_workflow_visualization(ui, engine);
        });
    }

    fn show_agent_panel(&mut self, ui: &mut egui::Ui, engine: &OrchestrationEngine) {
        ui.heading("Active Agents");
        
        ui.add_space(10.0);
        
        if ui.button("➕ Add Agent").clicked() {
            // TODO: Open agent creation dialog
        }
        
        ui.separator();
        
        egui::ScrollArea::vertical().show(ui, |ui| {
            let agents = engine.get_agents();
            
            for (idx, agent) in agents.iter().enumerate() {
                let is_selected = self.selected_agent == Some(idx);
                
                ui.push_id(idx, |ui| {
                    let frame_response = egui::Frame::none()
                        .fill(if is_selected {
                            ui.style().visuals.selection.bg_fill
                        } else {
                            Color32::TRANSPARENT
                        })
                        .inner_margin(8.0)
                        .show(ui, |ui| {
                            self.show_agent_card(ui, agent);
                        });
                    
                    if frame_response.response.clicked() {
                        self.selected_agent = Some(idx);
                    }
                });
                
                ui.add_space(4.0);
            }
        });
    }

    fn show_agent_card(&self, ui: &mut egui::Ui, agent: &Agent) {
        ui.horizontal(|ui| {
            // Status indicator
            let (status_color, status_text) = match &agent.status {
                AgentStatus::Idle => (Color32::GREEN, "●"),
                AgentStatus::Working { .. } => (Color32::YELLOW, "◐"),
                AgentStatus::Waiting { .. } => (Color32::BLUE, "◯"),
                AgentStatus::Error { .. } => (Color32::RED, "✕"),
                AgentStatus::Completed => (Color32::GRAY, "✓"),
            };
            
            ui.colored_label(status_color, status_text);
            
            ui.vertical(|ui| {
                ui.label(RichText::new(&agent.name).strong());
                ui.label(RichText::new(format!("{:?}", agent.role)).small());
                
                match &agent.status {
                    AgentStatus::Working { progress, .. } => {
                        ui.add(egui::ProgressBar::new(*progress).show_percentage());
                    }
                    AgentStatus::Error { retry_count } => {
                        ui.label(RichText::new(format!("Retries: {}", retry_count))
                            .color(Color32::RED)
                            .small());
                    }
                    _ => {}
                }
            });
        });
    }

    fn show_metrics_panel(&self, ui: &mut egui::Ui, engine: &OrchestrationEngine) {
        ui.heading("Performance Metrics");
        ui.separator();
        
        // Token usage chart
        ui.label("Token Usage by Agent");
        let agents = engine.get_agents();
        
        let bars: Vec<Bar> = agents
            .iter()
            .enumerate()
            .map(|(i, agent)| {
                Bar::new(i as f64, agent.metrics.total_tokens_used as f64)
                    .name(&agent.name)
            })
            .collect();
        
        let chart = BarChart::new(bars);
        
        Plot::new("token_usage")
            .height(150.0)
            .show(ui, |plot_ui| {
                plot_ui.bar_chart(chart);
            });
        
        ui.separator();
        
        // Success rate
        ui.label("Task Success Rate");
        for agent in &agents {
            let total = agent.metrics.total_tasks;
            let success_rate = if total > 0 {
                (agent.metrics.successful_tasks as f32 / total as f32) * 100.0
            } else {
                0.0
            };
            
            ui.horizontal(|ui| {
                ui.label(&agent.name);
                ui.add(egui::ProgressBar::new(success_rate / 100.0)
                    .text(format!("{:.1}%", success_rate)));
            });
        }
        
        ui.separator();
        
        // Cost tracking
        ui.label("Total Cost by Agent");
        let total_cost: f64 = agents.iter().map(|a| a.metrics.total_cost_usd).sum();
        ui.label(RichText::new(format!("Total: ${:.2}", total_cost)).strong());
        
        for agent in &agents {
            ui.horizontal(|ui| {
                ui.label(&agent.name);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(format!("${:.2}", agent.metrics.total_cost_usd));
                });
            });
        }
    }

    fn show_message_log(&mut self, ui: &mut egui::Ui, _engine: &OrchestrationEngine) {
        ui.horizontal(|ui| {
            ui.heading("Message Log");
            
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.checkbox(&mut self.message_filter.show_errors, "Errors");
                ui.checkbox(&mut self.message_filter.show_results, "Results");
                ui.checkbox(&mut self.message_filter.show_status, "Status");
                ui.checkbox(&mut self.message_filter.show_tasks, "Tasks");
                ui.label("Filter:");
            });
        });
        
        ui.separator();
        
        egui::ScrollArea::vertical()
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                // In a real implementation, we'd get messages from the message bus
                // For now, show placeholder
                ui.label("Message log will appear here...");
            });
    }

    fn show_workflow_visualization(&mut self, ui: &mut egui::Ui, engine: &OrchestrationEngine) {
        ui.heading("Workflow Orchestration");
        
        ui.horizontal(|ui| {
            if ui.button("▶ Execute Workflow").clicked() {
                // TODO: Open workflow selection dialog
            }
            
            if ui.button("⏸ Pause All").clicked() {
                // TODO: Pause all workflows
            }
            
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.checkbox(&mut self.show_metrics, "Show Metrics");
            });
        });
        
        ui.separator();
        
        // Get active workflows
        let workflows = engine.get_workflows();
        
        if workflows.is_empty() {
            ui.centered_and_justified(|ui| {
                ui.label("No active workflows. Click 'Execute Workflow' to start.");
            });
        } else {
            egui::ScrollArea::both().show(ui, |ui| {
                // In a real implementation, we'd render the workflow graph here
                ui.label("Workflow visualization will appear here...");
                
                for (workflow_id, state) in workflows {
                    ui.group(|ui| {
                        ui.label(format!("Workflow: {:?}", workflow_id));
                        ui.label(format!("State: {:?}", state));
                    });
                }
            });
        }
    }
}