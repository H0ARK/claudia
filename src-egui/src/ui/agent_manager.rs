use crate::models::{Agent, ModelType};
use crate::utils::ipc::IpcClient;
use egui::{Color32, Context, RichText};

pub struct AgentManagerView {
    agents: Vec<Agent>,
    selected_agent: Option<usize>,
    editing_agent: Option<Agent>,
    show_create_dialog: bool,
    new_agent_name: String,
    new_agent_role_type: String,
    new_agent_model: ModelType,
}

impl AgentManagerView {
    pub fn new() -> Self {
        Self {
            agents: Vec::new(),
            selected_agent: None,
            editing_agent: None,
            show_create_dialog: false,
            new_agent_name: String::new(),
            new_agent_role_type: "developer".to_string(),
            new_agent_model: ModelType::Claude4Sonnet,
        }
    }

    pub fn show(&mut self, ctx: &Context, _ipc_client: &mut IpcClient) {
        // Top toolbar
        egui::TopBottomPanel::top("agent_toolbar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button("➕ Create Agent").clicked() {
                    self.show_create_dialog = true;
                }
                
                if self.selected_agent.is_some() {
                    ui.separator();
                    
                    if ui.button("✏ Edit").clicked() {
                        if let Some(idx) = self.selected_agent {
                            self.editing_agent = Some(self.agents[idx].clone());
                        }
                    }
                    
                    if ui.button("🗑 Delete").clicked() {
                        // TODO: Confirm and delete
                    }
                    
                    ui.separator();
                    
                    if ui.button("▶ Execute").clicked() {
                        // TODO: Execute agent
                    }
                }
            });
        });

        // Main content
        egui::CentralPanel::default().show(ctx, |ui| {
            if self.agents.is_empty() {
                self.show_empty_state(ui);
            } else {
                self.show_agent_grid(ui);
            }
        });

        // Dialogs
        if self.show_create_dialog {
            self.show_create_agent_dialog(ctx);
        }

        if self.editing_agent.is_some() {
            self.show_edit_agent_dialog(ctx);
        }
    }

    fn show_empty_state(&self, ui: &mut egui::Ui) {
        ui.centered_and_justified(|ui| {
            ui.vertical_centered(|ui| {
                ui.label(RichText::new("🤖").size(64.0));
                ui.add_space(20.0);
                ui.heading("No Agents Yet");
                ui.label("Create your first agent to get started with orchestration.");
                ui.add_space(20.0);
                if ui.button("Create First Agent").clicked() {
                    // TODO: Open create dialog
                }
            });
        });
    }

    fn show_agent_grid(&mut self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            let available_width = ui.available_width();
            let card_width = 300.0;
            let spacing = 16.0;
            let _cards_per_row = ((available_width + spacing) / (card_width + spacing)).floor() as usize;
            
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(spacing, spacing);
                
                for (idx, agent) in self.agents.iter().enumerate() {
                    let is_selected = self.selected_agent == Some(idx);
                    
                    let frame_response = egui::Frame::none()
                        .fill(if is_selected {
                            ui.style().visuals.selection.bg_fill
                        } else {
                            ui.style().visuals.window_fill
                        })
                        .stroke(if is_selected {
                            egui::Stroke::new(2.0, ui.style().visuals.selection.stroke.color)
                        } else {
                            ui.style().visuals.window_stroke
                        })
                        .inner_margin(16.0)
                        .rounding(8.0)
                        .show(ui, |ui| {
                            ui.set_width(card_width - 32.0);
                            self.show_agent_card(ui, agent);
                        });
                    
                    let response = frame_response.response;
                    
                    if response.clicked() {
                        self.selected_agent = Some(idx);
                    }
                    
                    if response.double_clicked() {
                        self.editing_agent = Some(agent.clone());
                    }
                }
            });
        });
    }

    fn show_agent_card(&self, ui: &mut egui::Ui, agent: &Agent) {
        ui.vertical(|ui| {
            // Icon and name
            ui.horizontal(|ui| {
                ui.label(RichText::new("🤖").size(32.0));
                ui.vertical(|ui| {
                    ui.label(RichText::new(&agent.name).strong().size(16.0));
                    ui.label(RichText::new(format!("{:?}", agent.role))
                        .color(ui.style().visuals.weak_text_color())
                        .size(12.0));
                });
            });
            
            ui.add_space(8.0);
            ui.separator();
            ui.add_space(8.0);
            
            // Model
            ui.horizontal(|ui| {
                ui.label("Model:");
                ui.label(match &agent.model {
                    ModelType::Claude4Opus => "Claude 4 Opus",
                    ModelType::Claude4Sonnet => "Claude 4 Sonnet",
                    ModelType::Custom(name) => name,
                });
            });
            
            // Permissions
            ui.horizontal(|ui| {
                ui.label("Permissions:");
                
                let mut perms = Vec::new();
                if agent.permissions.file_read { perms.push("📖"); }
                if agent.permissions.file_write { perms.push("✏"); }
                if agent.permissions.network_access { perms.push("🌐"); }
                if agent.permissions.system_commands { perms.push("⚡"); }
                
                ui.label(perms.join(" "));
            });
            
            // Metrics
            ui.add_space(8.0);
            ui.separator();
            ui.add_space(8.0);
            
            ui.horizontal(|ui| {
                ui.label(format!("Tasks: {}", agent.metrics.total_tasks));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let success_rate = if agent.metrics.total_tasks > 0 {
                        (agent.metrics.successful_tasks as f32 / agent.metrics.total_tasks as f32) * 100.0
                    } else {
                        0.0
                    };
                    ui.label(RichText::new(format!("{:.0}%", success_rate))
                        .color(if success_rate >= 80.0 { Color32::GREEN } else if success_rate >= 50.0 { Color32::YELLOW } else { Color32::RED }));
                });
            });
            
            ui.label(RichText::new(format!("Cost: ${:.2}", agent.metrics.total_cost_usd))
                .size(11.0)
                .color(ui.style().visuals.weak_text_color()));
        });
    }

    fn show_create_agent_dialog(&mut self, ctx: &Context) {
        egui::Window::new("Create New Agent")
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Name:");
                    ui.text_edit_singleline(&mut self.new_agent_name);
                });
                
                ui.add_space(10.0);
                
                ui.horizontal(|ui| {
                    ui.label("Role:");
                    egui::ComboBox::from_label("")
                        .selected_text(&self.new_agent_role_type)
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut self.new_agent_role_type, "orchestrator".to_string(), "Orchestrator");
                            ui.selectable_value(&mut self.new_agent_role_type, "architect".to_string(), "Architect");
                            ui.selectable_value(&mut self.new_agent_role_type, "developer".to_string(), "Developer");
                            ui.selectable_value(&mut self.new_agent_role_type, "reviewer".to_string(), "Reviewer");
                            ui.selectable_value(&mut self.new_agent_role_type, "tester".to_string(), "Tester");
                            ui.selectable_value(&mut self.new_agent_role_type, "documentation".to_string(), "Documentation Writer");
                            ui.selectable_value(&mut self.new_agent_role_type, "devops".to_string(), "DevOps");
                            ui.selectable_value(&mut self.new_agent_role_type, "custom".to_string(), "Custom");
                        });
                });
                
                ui.add_space(10.0);
                
                ui.horizontal(|ui| {
                    ui.label("Model:");
                    ui.radio_value(&mut self.new_agent_model, ModelType::Claude4Sonnet, "Claude 4 Sonnet");
                    ui.radio_value(&mut self.new_agent_model, ModelType::Claude4Opus, "Claude 4 Opus");
                });
                
                ui.add_space(20.0);
                ui.separator();
                ui.add_space(10.0);
                
                ui.horizontal(|ui| {
                    if ui.button("Create").clicked() {
                        // TODO: Create agent
                        self.show_create_dialog = false;
                        self.new_agent_name.clear();
                    }
                    
                    if ui.button("Cancel").clicked() {
                        self.show_create_dialog = false;
                        self.new_agent_name.clear();
                    }
                });
            });
    }

    fn show_edit_agent_dialog(&mut self, ctx: &Context) {
        let mut should_close = false;
        let mut should_save = false;
        
        if let Some(agent) = &mut self.editing_agent {
            egui::Window::new("Edit Agent")
                .collapsible(false)
                .resizable(true)
                .default_width(600.0)
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Name:");
                        ui.text_edit_singleline(&mut agent.name);
                    });
                    
                    ui.add_space(10.0);
                    
                    ui.label("System Prompt:");
                    ui.add(
                        egui::TextEdit::multiline(&mut agent.system_prompt)
                            .desired_rows(10)
                            .desired_width(f32::INFINITY)
                    );
                    
                    ui.add_space(10.0);
                    
                    ui.collapsing("Permissions", |ui| {
                        ui.checkbox(&mut agent.permissions.file_read, "File Read");
                        ui.checkbox(&mut agent.permissions.file_write, "File Write");
                        ui.checkbox(&mut agent.permissions.network_access, "Network Access");
                        ui.checkbox(&mut agent.permissions.system_commands, "System Commands");
                        
                        ui.horizontal(|ui| {
                            ui.label("Max Tokens:");
                            if let Some(ref mut tokens) = agent.permissions.max_tokens {
                                ui.add(egui::DragValue::new(tokens).speed(1000));
                            }
                        });
                    });
                    
                    ui.add_space(20.0);
                    ui.separator();
                    ui.add_space(10.0);
                    
                    ui.horizontal(|ui| {
                        if ui.button("Save").clicked() {
                            // TODO: Save agent
                            should_save = true;
                            should_close = true;
                        }
                        
                        if ui.button("Cancel").clicked() {
                            should_close = true;
                        }
                    });
                });
        }
        
        if should_close {
            self.editing_agent = None;
        }
    }
}