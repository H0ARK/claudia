use egui::{Color32, Context, RichText, Ui};
// Removed unused import
use chrono::{DateTime, Utc};

/// Represents a worker in the UI
#[derive(Debug, Clone)]
pub struct WorkerInfo {
    pub id: String,
    pub worker_type: String,
    pub task_id: i64,
    pub status: String,
    pub started_at: DateTime<Utc>,
    pub metrics: WorkerMetrics,
}

/// Worker metrics for display
#[derive(Debug, Clone, Default)]
pub struct WorkerMetrics {
    pub messages_sent: u64,
    pub messages_received: u64,
    pub tasks_completed: u64,
    pub tasks_failed: u64,
    pub total_runtime_seconds: u64,
    pub error_count: u64,
}

/// Configuration for spawning a new worker
#[derive(Debug, Clone)]
pub struct WorkerSpawnConfig {
    pub worker_type: String,
    pub task_name: String,
    pub task_description: String,
    pub task_goal: String,
    pub model: Option<String>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
    pub custom_system_prompt: Option<String>,
    pub sandbox_profile: Option<String>,
}

impl Default for WorkerSpawnConfig {
    fn default() -> Self {
        Self {
            worker_type: "developer".to_string(),
            task_name: "New Task".to_string(),
            task_description: String::new(),
            task_goal: String::new(),
            model: None,
            temperature: Some(0.7),
            max_tokens: Some(4096),
            custom_system_prompt: None,
            sandbox_profile: None,
        }
    }
}

pub struct WorkerManagerView {
    workers: Vec<WorkerInfo>,
    selected_worker: Option<usize>,
    show_spawn_dialog: bool,
    spawn_config: WorkerSpawnConfig,
    global_metrics: WorkerMetrics,
    message_input: String,
    loading: bool,
    error_message: Option<String>,
    needs_refresh: bool,
}

impl WorkerManagerView {
    pub fn new() -> Self {
        Self {
            workers: Vec::new(),
            selected_worker: None,
            show_spawn_dialog: false,
            spawn_config: WorkerSpawnConfig::default(),
            global_metrics: WorkerMetrics::default(),
            message_input: String::new(),
            loading: false,
            error_message: None,
            needs_refresh: true,
        }
    }

    pub fn show(&mut self, ctx: &Context) {
        // Note: In a real implementation, this would integrate with the Tauri backend
        // For now, we'll show a mock UI that demonstrates the worker management interface
        
        // Top toolbar
        egui::TopBottomPanel::top("worker_toolbar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button("🚀 Spawn Worker").clicked() {
                    self.show_spawn_dialog = true;
                    self.spawn_config = WorkerSpawnConfig::default();
                }
                
                if ui.button("🔄 Refresh").clicked() {
                    self.needs_refresh = true;
                    // TODO: Call refresh_workers()
                }
                
                if self.selected_worker.is_some() {
                    ui.separator();
                    
                    if ui.button("🛑 Terminate").clicked() {
                        if let Some(idx) = self.selected_worker {
                            let worker_id = self.workers[idx].id.clone();
                            // TODO: Call terminate_worker(worker_id)
                        }
                    }
                    
                    if ui.button("📊 Metrics").clicked() {
                        // TODO: Show detailed metrics dialog
                    }
                }
                
                // Show global metrics
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(format!("Active Workers: {}", self.workers.len()));
                    ui.separator();
                    ui.label(format!("Total Tasks: {}", self.global_metrics.tasks_completed + self.global_metrics.tasks_failed));
                    
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

        // Main content area
        egui::CentralPanel::default().show(ctx, |ui| {
            if self.workers.is_empty() && !self.loading {
                self.show_empty_state(ui);
            } else {
                self.show_worker_list(ui);
            }
        });

        // Spawn worker dialog
        if self.show_spawn_dialog {
            self.show_spawn_worker_dialog(ctx);
        }
    }

    fn show_empty_state(&mut self, ui: &mut Ui) {
        ui.centered_and_justified(|ui| {
            ui.vertical_centered(|ui| {
                ui.label(RichText::new("🚀").size(64.0));
                ui.add_space(20.0);
                ui.heading("No Active Workers");
                ui.label("Spawn your first worker to start distributed task execution.");
                ui.add_space(20.0);
                if ui.button("Spawn First Worker").clicked() {
                    self.show_spawn_dialog = true;
                    self.spawn_config = WorkerSpawnConfig::default();
                }
            });
        });
    }

    fn show_worker_list(&mut self, ui: &mut Ui) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            // Table header
            ui.horizontal(|ui| {
                ui.label("Worker ID");
                ui.separator();
                ui.label("Type");
                ui.separator();
                ui.label("Status");
                ui.separator();
                ui.label("Task ID");
                ui.separator();
                ui.label("Started");
                ui.separator();
                ui.label("Metrics");
            });
            
            ui.separator();
            
            // Worker rows
            for (idx, worker) in self.workers.iter().enumerate() {
                let is_selected = self.selected_worker == Some(idx);
                
                let response = ui.selectable_label(is_selected, "");
                
                ui.horizontal(|ui| {
                    ui.label(&worker.id);
                    ui.separator();
                    ui.label(self.format_worker_type(&worker.worker_type));
                    ui.separator();
                    ui.colored_label(
                        self.status_color(&worker.status),
                        self.format_status(&worker.status)
                    );
                    ui.separator();
                    ui.label(worker.task_id.to_string());
                    ui.separator();
                    ui.label(worker.started_at.format("%H:%M:%S").to_string());
                    ui.separator();
                    ui.label(format!(
                        "Sent: {} | Received: {} | Completed: {} | Failed: {}",
                        worker.metrics.messages_sent,
                        worker.metrics.messages_received,
                        worker.metrics.tasks_completed,
                        worker.metrics.tasks_failed
                    ));
                });
                
                if response.clicked() {
                    self.selected_worker = Some(idx);
                }
                
                ui.separator();
            }
        });
        
        // Worker details panel
        if let Some(idx) = self.selected_worker {
            if idx < self.workers.len() {
                self.show_worker_details(ui, &self.workers[idx].clone());
            }
        }
    }

    fn show_worker_details(&mut self, ui: &mut Ui, worker: &WorkerInfo) {
        ui.separator();
        ui.heading(format!("Worker Details: {}", worker.id));
        
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.label(format!("Type: {}", self.format_worker_type(&worker.worker_type)));
                ui.label(format!("Status: {}", self.format_status(&worker.status)));
                ui.label(format!("Task ID: {}", worker.task_id));
                ui.label(format!("Started: {}", worker.started_at.format("%Y-%m-%d %H:%M:%S")));
                ui.label(format!("Runtime: {}s", worker.metrics.total_runtime_seconds));
            });
            
            ui.separator();
            
            ui.vertical(|ui| {
                ui.label("Metrics:");
                ui.label(format!("Messages Sent: {}", worker.metrics.messages_sent));
                ui.label(format!("Messages Received: {}", worker.metrics.messages_received));
                ui.label(format!("Tasks Completed: {}", worker.metrics.tasks_completed));
                ui.label(format!("Tasks Failed: {}", worker.metrics.tasks_failed));
                ui.label(format!("Errors: {}", worker.metrics.error_count));
            });
        });
        
        ui.separator();
        
        // Message input
        ui.horizontal(|ui| {
            ui.label("Send Message:");
            ui.text_edit_singleline(&mut self.message_input);
            if ui.button("Send").clicked() && !self.message_input.is_empty() {
                let message = self.message_input.clone();
                self.message_input.clear();
                // TODO: Call send_message_to_worker(worker.id, message)
            }
        });
    }

    fn show_spawn_worker_dialog(&mut self, ctx: &Context) {
        egui::Window::new("Spawn New Worker")
            .collapsible(false)
            .resizable(true)
            .default_width(500.0)
            .show(ctx, |ui| {
                ui.vertical(|ui| {
                    // Worker type selection
                    ui.horizontal(|ui| {
                        ui.label("Worker Type:");
                        egui::ComboBox::from_label("")
                            .selected_text(&self.spawn_config.worker_type)
                            .show_ui(ui, |ui| {
                                ui.selectable_value(&mut self.spawn_config.worker_type, "developer".to_string(), "Developer");
                                ui.selectable_value(&mut self.spawn_config.worker_type, "tester".to_string(), "Tester");
                                ui.selectable_value(&mut self.spawn_config.worker_type, "reviewer".to_string(), "Reviewer");
                                ui.selectable_value(&mut self.spawn_config.worker_type, "documentation".to_string(), "Documentation");
                            });
                    });
                    
                    // Task details
                    ui.horizontal(|ui| {
                        ui.label("Task Name:");
                        ui.text_edit_singleline(&mut self.spawn_config.task_name);
                    });
                    
                    ui.horizontal(|ui| {
                        ui.label("Description:");
                        ui.text_edit_multiline(&mut self.spawn_config.task_description);
                    });
                    
                    ui.horizontal(|ui| {
                        ui.label("Goal:");
                        ui.text_edit_multiline(&mut self.spawn_config.task_goal);
                    });
                    
                    // Model configuration
                    ui.collapsing("Advanced Configuration", |ui| {
                        ui.horizontal(|ui| {
                            ui.label("Model:");
                            let model_text = self.spawn_config.model.get_or_insert_with(|| "claude-3-5-sonnet-20241022".to_string());
                            ui.text_edit_singleline(model_text);
                        });
                        
                        ui.horizontal(|ui| {
                            ui.label("Temperature:");
                            let mut temp = self.spawn_config.temperature.unwrap_or(0.7);
                            ui.add(egui::Slider::new(&mut temp, 0.0..=1.0));
                            self.spawn_config.temperature = Some(temp);
                        });
                        
                        ui.horizontal(|ui| {
                            ui.label("Max Tokens:");
                            let mut tokens = self.spawn_config.max_tokens.unwrap_or(4096);
                            ui.add(egui::Slider::new(&mut tokens, 1..=8192));
                            self.spawn_config.max_tokens = Some(tokens);
                        });
                        
                        ui.horizontal(|ui| {
                            ui.label("Custom System Prompt:");
                            let prompt_text = self.spawn_config.custom_system_prompt.get_or_insert_with(String::new);
                            ui.text_edit_multiline(prompt_text);
                        });
                        
                        ui.horizontal(|ui| {
                            ui.label("Sandbox Profile:");
                            let profile_text = self.spawn_config.sandbox_profile.get_or_insert_with(String::new);
                            ui.text_edit_singleline(profile_text);
                        });
                    });
                    
                    ui.separator();
                    
                    // Buttons
                    ui.horizontal(|ui| {
                        if ui.button("Spawn Worker").clicked() {
                            // TODO: Call spawn_worker with config
                            // For now, just close the dialog
                            self.show_spawn_dialog = false;
                        }
                        
                        if ui.button("Cancel").clicked() {
                            self.show_spawn_dialog = false;
                        }
                    });
                });
            });
    }

    fn format_worker_type(&self, worker_type: &str) -> String {
        match worker_type {
            "developer" => "👨‍💻 Developer".to_string(),
            "tester" => "🧪 Tester".to_string(),
            "reviewer" => "👁️ Reviewer".to_string(),
            "documentation" => "📝 Documentation".to_string(),
            _ if worker_type.starts_with("custom_") => "⚙️ Custom".to_string(),
            _ => worker_type.to_string(),
        }
    }

    fn format_status(&self, status: &str) -> String {
        match status {
            "starting" => "🟡 Starting".to_string(),
            "running" => "🟢 Running".to_string(),
            "idle" => "🔵 Idle".to_string(),
            "busy" => "🟠 Busy".to_string(),
            "stopped" => "⚫ Stopped".to_string(),
            s if s.starts_with("failed") => format!("🔴 {}", s),
            _ => status.to_string(),
        }
    }

    fn status_color(&self, status: &str) -> Color32 {
        match status {
            "starting" => Color32::from_rgb(255, 255, 0),
            "running" => Color32::from_rgb(0, 255, 0),
            "idle" => Color32::from_rgb(0, 0, 255),
            "busy" => Color32::from_rgb(255, 165, 0),
            "stopped" => Color32::from_rgb(128, 128, 128),
            s if s.starts_with("failed") => Color32::from_rgb(255, 0, 0),
            _ => Color32::WHITE,
        }
    }
}

// TODO: Implement integration with Tauri backend
// These functions would call the Tauri commands we created
/*
async fn refresh_workers() -> Result<Vec<WorkerInfo>, String> {
    // Call get_active_workers Tauri command
    todo!()
}

async fn spawn_worker(config: WorkerSpawnConfig) -> Result<String, String> {
    // Call spawn_worker Tauri command
    todo!()
}

async fn terminate_worker(worker_id: String) -> Result<(), String> {
    // Call terminate_worker Tauri command
    todo!()
}

async fn send_message_to_worker(worker_id: String, message: String) -> Result<(), String> {
    // Call send_message_to_worker Tauri command
    todo!()
}
*/ 