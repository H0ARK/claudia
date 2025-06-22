use crate::models::TauriAgent;
use crate::orchestration::{AgentBuilder, GeneratedAgent};
use crate::utils::BackendBridge;
use egui::{Color32, Context, RichText};
use std::sync::Arc;

pub struct AgentManagerView {
    agents: Vec<TauriAgent>,
    selected_agent: Option<usize>,
    editing_agent: Option<TauriAgent>,
    show_create_dialog: bool,
    show_ai_builder_dialog: bool,
    new_agent: TauriAgent,
    ai_description: String,
    generated_agent: Option<GeneratedAgent>,
    loading: bool,
    error_message: Option<String>,
    success_message: Option<String>,
    needs_refresh: bool,
}

impl AgentManagerView {
    pub fn new() -> Self {
        Self {
            agents: Vec::new(),
            selected_agent: None,
            editing_agent: None,
            show_create_dialog: false,
            show_ai_builder_dialog: false,
            new_agent: TauriAgent::default(),
            ai_description: String::new(),
            generated_agent: None,
            loading: false,
            error_message: None,
            success_message: None,
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
                    println!("Create Agent button clicked!");
                    self.show_create_dialog = true;
                    self.new_agent = TauriAgent::default();
                }
                
                if ui.button("🤖 AI Builder").clicked() {
                    println!("AI Builder button clicked!");
                    self.show_ai_builder_dialog = true;
                    self.ai_description.clear();
                    self.generated_agent = None;
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
                    
                    if let Some(success) = &self.success_message {
                        ui.colored_label(Color32::GREEN, format!("✅ {}", success));
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
        
        if self.show_ai_builder_dialog {
            self.show_ai_builder_dialog(ctx, backend_bridge);
        }
    }

    fn load_agents(&mut self, backend_bridge: &Arc<BackendBridge>) {
        self.loading = true;
        self.error_message = None;
        
        match backend_bridge.list_agents() {
            Ok(agents) => {
                println!("Loaded {} agents", agents.len());
                self.agents = agents;
                self.loading = false;
            }
            Err(e) => {
                println!("Error loading agents: {}", e);
                self.error_message = Some(format!("Failed to load agents: {}", e));
                self.loading = false;
            }
        }
    }

    fn create_agent(&mut self, backend_bridge: &Arc<BackendBridge>, agent: TauriAgent) {
        println!("create_agent called with agent: {:?}", agent.name);
        match backend_bridge.create_agent(&agent) {
            Ok(id) => {
                println!("Agent created successfully with ID: {}", id);
                self.needs_refresh = true;
                self.show_create_dialog = false;
            }
            Err(e) => {
                println!("Failed to create agent: {}", e);
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
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
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
                    let create_enabled = !self.new_agent.name.is_empty();
                    ui.add_enabled_ui(create_enabled, |ui| {
                        if ui.button("Create").clicked() {
                            println!("Create button in dialog clicked! Agent name: {}", self.new_agent.name);
                            let agent = self.new_agent.clone();
                            self.create_agent(backend_bridge, agent);
                        }
                    });
                    
                    if ui.button("Cancel").clicked() {
                        println!("Cancel button clicked!");
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
    
    fn show_ai_builder_dialog(&mut self, ctx: &Context, backend_bridge: &Arc<BackendBridge>) {
        egui::Window::new("AI Agent Builder")
            .collapsible(false)
            .resizable(true)
            .default_width(700.0)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.heading("Create Agent with AI");
                ui.label("Describe the agent you want to create, and AI will generate it for you.");
                ui.separator();
                
                ui.label("Agent Description:");
                ui.add(
                    egui::TextEdit::multiline(&mut self.ai_description)
                        .desired_rows(5)
                        .desired_width(f32::INFINITY)
                        .hint_text("Example: I need an agent that can review code for security vulnerabilities and suggest improvements...")
                );
                
                ui.add_space(10.0);
                
                // Show generated agent if available
                if let Some(generated) = &self.generated_agent {
                    ui.separator();
                    ui.heading("Generated Agent");
                    
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(&generated.agent.icon).size(32.0));
                        ui.vertical(|ui| {
                            ui.label(RichText::new(&generated.agent.name).strong().size(16.0));
                            ui.label(RichText::new(generated.agent.display_model_name())
                                .color(ui.style().visuals.weak_text_color())
                                .size(12.0));
                        });
                    });
                    
                    ui.add_space(10.0);
                    
                    ui.collapsing("System Prompt", |ui| {
                        ui.label(&generated.agent.system_prompt);
                    });
                    
                    ui.collapsing("Reasoning", |ui| {
                        ui.label(&generated.reasoning);
                    });
                    
                    ui.collapsing("Suggested Workflows", |ui| {
                        for workflow in &generated.suggested_workflows {
                            ui.label(format!("• {}", workflow));
                        }
                    });
                    
                    ui.add_space(10.0);
                    ui.separator();
                }
                
                ui.add_space(10.0);
                
                ui.horizontal(|ui| {
                    let generate_enabled = !self.ai_description.is_empty() && !self.loading;
                    ui.add_enabled_ui(generate_enabled, |ui| {
                        if ui.button("🤖 Generate Agent").clicked() {
                            self.generate_agent_from_description(backend_bridge);
                        }
                    });
                    
                    if self.generated_agent.is_some() {
                        if ui.button("💾 Save Agent").clicked() {
                            self.save_generated_agent(backend_bridge);
                        }
                    }
                    
                    if ui.button("Cancel").clicked() {
                        self.show_ai_builder_dialog = false;
                        self.ai_description.clear();
                        self.generated_agent = None;
                    }
                });
                
                if self.loading {
                    ui.add_space(10.0);
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.label("Generating agent configuration...");
                    });
                }
            });
    }
    
    fn generate_agent_from_description(&mut self, backend_bridge: &Arc<BackendBridge>) {
        self.loading = true;
        self.error_message = None;
        
        // Create a simple generated agent based on the description
        // In a real implementation, this would call the AgentBuilder with Claude API
        let agent_builder = AgentBuilder::new(backend_bridge.clone());
        
        // For now, create a mock generated agent
        // In production, this would be: agent_builder.build_agent_from_description(&self.ai_description).await
        let generated = self.create_mock_generated_agent(&self.ai_description);
        
        self.generated_agent = Some(generated);
        self.loading = false;
    }
    
    fn create_mock_generated_agent(&self, description: &str) -> GeneratedAgent {
        // This is a simplified version - in production, the AgentBuilder would use Claude
        let name = if description.to_lowercase().contains("security") {
            "Security Analyst"
        } else if description.to_lowercase().contains("test") {
            "Test Engineer"
        } else if description.to_lowercase().contains("data") {
            "Data Processor"
        } else {
            "Custom Agent"
        }.to_string();
        
        let icon = if description.to_lowercase().contains("security") {
            "🔒"
        } else if description.to_lowercase().contains("test") {
            "🧪"
        } else if description.to_lowercase().contains("data") {
            "📊"
        } else {
            "🤖"
        }.to_string();
        
        let agent = TauriAgent {
            id: None,
            name: name.clone(),
            icon,
            system_prompt: format!(
                "You are {}, an AI agent specialized based on the following requirements:\n\n{}\n\nProvide expert assistance in your domain while maintaining high quality standards.",
                name,
                description
            ),
            default_task: Some(format!("Execute {} tasks", name.to_lowercase())),
            model: "claude-3-sonnet-20240229".to_string(),
            sandbox_enabled: false,
            enable_file_read: true,
            enable_file_write: description.to_lowercase().contains("generate") || description.to_lowercase().contains("create"),
            enable_network: description.to_lowercase().contains("api") || description.to_lowercase().contains("deploy"),
            enable_system_commands: description.to_lowercase().contains("build") || description.to_lowercase().contains("test"),
            custom_instructions: Some(format!("Specialized for: {}", description)),
            sandbox_profile_id: None,
            created_at: None,
            updated_at: None,
        };
        
        GeneratedAgent {
            agent,
            reasoning: "Agent configured based on the description provided. Permissions and model selected to match the required capabilities.".to_string(),
            suggested_workflows: vec![
                format!("{} Workflow", name),
                "Automated Pipeline".to_string(),
            ],
        }
    }
    
    fn save_generated_agent(&mut self, backend_bridge: &Arc<BackendBridge>) {
        if let Some(generated) = &self.generated_agent {
            match backend_bridge.create_agent(&generated.agent) {
                Ok(_) => {
                    self.success_message = Some(format!("Agent '{}' created successfully!", generated.agent.name));
                    self.needs_refresh = true;
                    self.show_ai_builder_dialog = false;
                    self.ai_description.clear();
                    self.generated_agent = None;
                    
                    // Clear success message after 3 seconds
                    // In a real app, you'd use a timer/scheduler for this
                }
                Err(e) => {
                    self.error_message = Some(format!("Failed to save agent: {}", e));
                }
            }
        }
    }
}