use crate::models::TauriAgent;
use crate::utils::BackendBridge;
use egui::{Color32, Context, RichText};
use std::sync::Arc;

pub struct AgentManagerView {
    agents: Vec<TauriAgent>,
    selected_agent: Option<usize>,
    editing_agent: Option<TauriAgent>,
    show_create_dialog: bool,
    new_agent: TauriAgent,
    loading: bool,
    error_message: Option<String>,
    needs_refresh: bool,
}

impl AgentManagerView {
    pub fn new() -> Self {
        Self {
            agents: Vec::new(),
            selected_agent: None,
            editing_agent: None,
            show_create_dialog: false,
            new_agent: TauriAgent::default(),
            loading: false,
            error_message: None,
            needs_refresh: true,
        }
    }

    pub fn show(&mut self, ctx: &Context, backend_bridge: &Arc<BackendBridge>) {
        // Load agents on first render or when refresh is needed
        if self.needs_refresh {
            self.load_agents(backend_bridge);
            self.needs_refresh = false;
        }

        // Top toolbar
        egui::TopBottomPanel::top("agent_toolbar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button("➕ Create Agent").clicked() {
                    self.show_create_dialog = true;
                    self.new_agent = TauriAgent::default();
                }
                
                if ui.button("🔄 Refresh").clicked() {
                    self.needs_refresh = true;
                }
                
                if self.selected_agent.is_some() {
                    ui.separator();
                    
                    if ui.button("✏️ Edit").clicked() {
                        if let Some(idx) = self.selected_agent {
                            self.editing_agent = Some(self.agents[idx].clone());
                        }
                    }
                    
                    if ui.button("🗑️ Delete").clicked() {
                        if let Some(idx) = self.selected_agent {
                            if let Some(agent_id) = self.agents[idx].id {
                                self.delete_agent(backend_bridge, agent_id);
                            }
                        }
                    }
                    
                    ui.separator();
                    
                    if ui.button("▶️ Execute").clicked() {
                        // TODO: Show execute dialog
                    }
                }
                
                // Show loading/error status
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if self.loading {
                        ui.spinner();
                        ui.label("Loading...");
                    }
                    
                    if let Some(error) = &self.error_message {
                        ui.colored_label(Color32::RED, format!("❌ {}", error));
                    }
                });
            });
        });

        // Main content
        egui::CentralPanel::default().show(ctx, |ui| {
            if self.agents.is_empty() && !self.loading {
                self.show_empty_state(ui);
            } else {
                self.show_agent_grid(ui);
            }
        });

        // Dialogs
        if self.show_create_dialog {
            self.show_create_agent_dialog(ctx, backend_bridge);
        }

        if let Some(_) = &self.editing_agent {
            self.show_edit_agent_dialog(ctx, backend_bridge);
        }
    }

    fn load_agents(&mut self, backend_bridge: &Arc<BackendBridge>) {
        self.loading = true;
        self.error_message = None;
        
        match backend_bridge.list_agents() {
            Ok(agents) => {
                self.agents = agents;
                self.loading = false;
            }
            Err(e) => {
                self.error_message = Some(format!("Failed to load agents: {}", e));
                self.loading = false;
            }
        }
    }

    fn create_agent(&mut self, backend_bridge: &Arc<BackendBridge>, agent: TauriAgent) {
        match backend_bridge.create_agent(&agent) {
            Ok(_) => {
                self.needs_refresh = true;
                self.show_create_dialog = false;
            }
            Err(e) => {
                self.error_message = Some(format!("Failed to create agent: {}", e));
            }
        }
    }

    fn update_agent(&mut self, backend_bridge: &Arc<BackendBridge>, agent: TauriAgent) {
        match backend_bridge.update_agent(&agent) {
            Ok(_) => {
                self.needs_refresh = true;
                self.editing_agent = None;
            }
            Err(e) => {
                self.error_message = Some(format!("Failed to update agent: {}", e));
            }
        }
    }

    fn delete_agent(&mut self, backend_bridge: &Arc<BackendBridge>, agent_id: i64) {
        match backend_bridge.delete_agent(agent_id) {
            Ok(_) => {
                self.needs_refresh = true;
                self.selected_agent = None;
            }
            Err(e) => {
                self.error_message = Some(format!("Failed to delete agent: {}", e));
            }
        }
    }

    fn show_empty_state(&mut self, ui: &mut egui::Ui) {
        ui.centered_and_justified(|ui| {
            ui.vertical_centered(|ui| {
                ui.label(RichText::new("🤖").size(64.0));
                ui.add_space(20.0);
                ui.heading("No Agents Yet");
                ui.label("Create your first agent to get started with orchestration.");
                ui.add_space(20.0);
                if ui.button("Create First Agent").clicked() {
                    self.show_create_dialog = true;
                    self.new_agent = TauriAgent::default();
                }
            });
        });
    }

    fn show_agent_grid(&mut self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            let available_width = ui.available_width();
            let card_width = 300.0;
            let spacing = 16.0;
            
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

    fn show_agent_card(&self, ui: &mut egui::Ui, agent: &TauriAgent) {
        ui.vertical(|ui| {
            // Icon and name
            ui.horizontal(|ui| {
                ui.label(RichText::new(&agent.icon).size(32.0));
                ui.vertical(|ui| {
                    ui.label(RichText::new(&agent.name).strong().size(16.0));
                    ui.label(RichText::new(agent.display_model_name())
                        .color(ui.style().visuals.weak_text_color())
                        .size(12.0));
                });
            });
            
            ui.add_space(8.0);
            ui.separator();
            ui.add_space(8.0);
            
            // Permissions
            ui.horizontal(|ui| {
                ui.label("Permissions:");
                let perms = agent.get_permissions_summary();
                ui.label(perms.join(" "));
            });
            
            // System prompt preview
            if !agent.system_prompt.is_empty() {
                ui.add_space(8.0);
                ui.separator();
                ui.add_space(8.0);
                
                let preview = if agent.system_prompt.len() > 100 {
                    format!("{}...", &agent.system_prompt[..100])
                } else {
                    agent.system_prompt.clone()
                };
                
                ui.label(RichText::new(preview)
                    .size(11.0)
                    .color(ui.style().visuals.weak_text_color()));
            }
        });
    }

    fn show_create_agent_dialog(&mut self, ctx: &Context, backend_bridge: &Arc<BackendBridge>) {
        egui::Window::new("Create New Agent")
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Name:");
                    ui.text_edit_singleline(&mut self.new_agent.name);
                });
                
                ui.add_space(10.0);
                
                ui.horizontal(|ui| {
                    ui.label("Icon:");
                    ui.text_edit_singleline(&mut self.new_agent.icon);
                    ui.label("(emoji)");
                });
                
                ui.add_space(10.0);
                
                ui.horizontal(|ui| {
                    ui.label("Model:");
                    egui::ComboBox::from_label("")
                        .selected_text(self.new_agent.display_model_name())
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut self.new_agent.model, "claude-3-opus-20240229".to_string(), "Claude 3 Opus");
                            ui.selectable_value(&mut self.new_agent.model, "claude-3-sonnet-20240229".to_string(), "Claude 3 Sonnet");
                            ui.selectable_value(&mut self.new_agent.model, "claude-3-haiku-20240307".to_string(), "Claude 3 Haiku");
                        });
                });
                
                ui.add_space(10.0);
                
                ui.label("System Prompt:");
                ui.add(
                    egui::TextEdit::multiline(&mut self.new_agent.system_prompt)
                        .desired_rows(5)
                        .desired_width(400.0)
                );
                
                ui.add_space(10.0);
                
                ui.collapsing("Permissions", |ui| {
                    ui.checkbox(&mut self.new_agent.enable_file_read, "File Read");
                    ui.checkbox(&mut self.new_agent.enable_file_write, "File Write");
                    ui.checkbox(&mut self.new_agent.enable_network, "Network Access");
                    ui.checkbox(&mut self.new_agent.enable_system_commands, "System Commands");
                    ui.checkbox(&mut self.new_agent.sandbox_enabled, "Enable Sandbox");
                });
                
                ui.add_space(20.0);
                ui.separator();
                ui.add_space(10.0);
                
                ui.horizontal(|ui| {
                    if ui.button("Create").clicked() && !self.new_agent.name.is_empty() {
                        let agent = self.new_agent.clone();
                        self.create_agent(backend_bridge, agent);
                    }
                    
                    if ui.button("Cancel").clicked() {
                        self.show_create_dialog = false;
                        self.new_agent = TauriAgent::default();
                    }
                });
            });
    }

    fn show_edit_agent_dialog(&mut self, ctx: &Context, backend_bridge: &Arc<BackendBridge>) {
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
                    
                    ui.horizontal(|ui| {
                        ui.label("Icon:");
                        ui.text_edit_singleline(&mut agent.icon);
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
                        ui.checkbox(&mut agent.enable_file_read, "File Read");
                        ui.checkbox(&mut agent.enable_file_write, "File Write");
                        ui.checkbox(&mut agent.enable_network, "Network Access");
                        ui.checkbox(&mut agent.enable_system_commands, "System Commands");
                        ui.checkbox(&mut agent.sandbox_enabled, "Enable Sandbox");
                    });
                    
                    ui.add_space(20.0);
                    ui.separator();
                    ui.add_space(10.0);
                    
                    ui.horizontal(|ui| {
                        if ui.button("Save").clicked() {
                            should_save = true;
                            should_close = true;
                        }
                        
                        if ui.button("Cancel").clicked() {
                            should_close = true;
                        }
                    });
                });
        }
        
        if should_save {
            if let Some(agent) = self.editing_agent.clone() {
                self.update_agent(backend_bridge, agent);
            }
        }
        
        if should_close {
            self.editing_agent = None;
        }
    }
}