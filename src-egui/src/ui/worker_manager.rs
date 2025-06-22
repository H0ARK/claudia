use crate::models::TauriAgent;
use crate::utils::BackendBridge;
use egui::{Color32, Context, RichText};
use std::collections::HashMap;
use std::process::{Child, Command};
use std::sync::{Arc, Mutex};
use std::time::Instant;

pub struct WorkerManagerView {
    workers: HashMap<u64, WorkerInfo>,
    selected_worker: Option<u64>,
    available_agents: Vec<TauriAgent>,
    show_spawn_dialog: bool,
    selected_agent_id: Option<i64>,
    worker_args: String,
    error_message: Option<String>,
    backend_bridge: Option<Arc<BackendBridge>>,
    next_worker_id: u64,
}

struct WorkerInfo {
    id: u64,
    agent_name: String,
    agent_id: i64,
    process: Arc<Mutex<Child>>,
    status: WorkerStatus,
    started_at: Instant,
    last_heartbeat: Instant,
    logs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
enum WorkerStatus {
    Running,
    Stopped,
    Crashed,
    Unknown,
}

impl WorkerManagerView {
    pub fn new() -> Self {
        Self {
            workers: HashMap::new(),
            selected_worker: None,
            available_agents: Vec::new(),
            show_spawn_dialog: false,
            selected_agent_id: None,
            worker_args: String::new(),
            error_message: None,
            backend_bridge: None,
            next_worker_id: 1,
        }
    }

    pub fn show(&mut self, ctx: &Context, backend_bridge: &Arc<BackendBridge>) {
        // Store backend bridge reference
        if self.backend_bridge.is_none() {
            self.backend_bridge = Some(backend_bridge.clone());
            self.load_available_agents(backend_bridge);
        }

        // Top toolbar
        egui::TopBottomPanel::top("worker_toolbar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button("🚀 Spawn Worker").clicked() {
                    self.show_spawn_dialog = true;
                }
                
                if ui.button("🔄 Refresh").clicked() {
                    self.refresh_worker_status();
                }
                
                if let Some(worker_id) = self.selected_worker {
                    ui.separator();
                    
                    if ui.button("⏹️ Stop").clicked() {
                        self.stop_worker(worker_id);
                    }
                    
                    if ui.button("🔄 Restart").clicked() {
                        self.restart_worker(worker_id);
                    }
                    
                    if ui.button("📝 View Logs").clicked() {
                        // TODO: Show logs window
                    }
                }
                
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(format!("Active Workers: {}", self.workers.len()));
                    
                    if let Some(error) = &self.error_message {
                        ui.colored_label(Color32::RED, format!("❌ {}", error));
                    }
                });
            });
        });

        // Main content - split view
        egui::SidePanel::left("worker_list")
            .default_width(300.0)
            .show(ctx, |ui| {
                self.show_worker_list(ui);
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(worker_id) = self.selected_worker {
                self.show_worker_details(ui, worker_id);
            } else {
                self.show_empty_state(ui);
            }
        });

        // Spawn worker dialog
        if self.show_spawn_dialog {
            self.show_spawn_worker_dialog(ctx);
        }
    }

    fn load_available_agents(&mut self, backend_bridge: &Arc<BackendBridge>) {
        match backend_bridge.list_agents() {
            Ok(agents) => {
                self.available_agents = agents;
            }
            Err(e) => {
                self.error_message = Some(format!("Failed to load agents: {}", e));
            }
        }
    }

    fn show_worker_list(&mut self, ui: &mut egui::Ui) {
        ui.heading("Workers");
        ui.separator();
        
        egui::ScrollArea::vertical().show(ui, |ui| {
            let worker_ids: Vec<u64> = self.workers.keys().copied().collect();
            
            for worker_id in worker_ids {
                if let Some(worker) = self.workers.get(&worker_id) {
                    let is_selected = self.selected_worker == Some(worker_id);
                    
                    let response = ui.add(
                        egui::SelectableLabel::new(is_selected, format!(
                            "{} {} - {}",
                            match worker.status {
                                WorkerStatus::Running => "🟢",
                                WorkerStatus::Stopped => "🔴",
                                WorkerStatus::Crashed => "💥",
                                WorkerStatus::Unknown => "❓",
                            },
                            worker.agent_name,
                            format!("Worker #{}", worker.id)
                        ))
                    );
                    
                    if response.clicked() {
                        self.selected_worker = Some(worker_id);
                    }
                }
            }
        });
    }

    fn show_worker_details(&self, ui: &mut egui::Ui, worker_id: u64) {
        if let Some(worker) = self.workers.get(&worker_id) {
            ui.heading(format!("Worker #{} - {}", worker.id, worker.agent_name));
            ui.separator();
            
            ui.horizontal(|ui| {
                ui.label("Status:");
                let status_color = match worker.status {
                    WorkerStatus::Running => Color32::GREEN,
                    WorkerStatus::Stopped => Color32::RED,
                    WorkerStatus::Crashed => Color32::DARK_RED,
                    WorkerStatus::Unknown => Color32::GRAY,
                };
                ui.colored_label(status_color, format!("{:?}", worker.status));
            });
            
            ui.horizontal(|ui| {
                ui.label("Running for:");
                let duration = worker.started_at.elapsed();
                ui.label(format!("{:02}:{:02}:{:02}", 
                    duration.as_secs() / 3600,
                    (duration.as_secs() % 3600) / 60,
                    duration.as_secs() % 60
                ));
            });
            
            ui.horizontal(|ui| {
                ui.label("Last heartbeat:");
                let heartbeat_ago = worker.last_heartbeat.elapsed().as_secs();
                ui.label(format!("{} seconds ago", heartbeat_ago));
            });
            
            ui.add_space(20.0);
            ui.separator();
            ui.add_space(10.0);
            
            ui.heading("Logs");
            
            egui::ScrollArea::vertical()
                .auto_shrink([false; 2])
                .show(ui, |ui| {
                    for log in &worker.logs {
                        ui.label(log);
                    }
                });
        }
    }

    fn show_empty_state(&self, ui: &mut egui::Ui) {
        ui.centered_and_justified(|ui| {
            ui.vertical_centered(|ui| {
                ui.label(RichText::new("🚀").size(64.0));
                ui.add_space(20.0);
                ui.heading("No Workers Running");
                ui.label("Spawn a worker to begin task execution.");
            });
        });
    }

    fn show_spawn_worker_dialog(&mut self, ctx: &Context) {
        egui::Window::new("Spawn Worker")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.heading("Select Agent");
                ui.separator();
                
                egui::ComboBox::from_label("Agent")
                    .selected_text(
                        self.selected_agent_id
                            .and_then(|id| self.available_agents.iter().find(|a| a.id == Some(id)))
                            .map(|a| a.name.clone())
                            .unwrap_or_else(|| "Select an agent...".to_string())
                    )
                    .show_ui(ui, |ui| {
                        for agent in &self.available_agents {
                            if let Some(agent_id) = agent.id {
                                ui.selectable_value(
                                    &mut self.selected_agent_id,
                                    Some(agent_id),
                                    &agent.name
                                );
                            }
                        }
                    });
                
                ui.add_space(10.0);
                
                ui.label("Additional Arguments:");
                ui.text_edit_singleline(&mut self.worker_args);
                
                ui.add_space(20.0);
                ui.separator();
                ui.add_space(10.0);
                
                ui.horizontal(|ui| {
                    if ui.button("Spawn").clicked() {
                        if let Some(agent_id) = self.selected_agent_id {
                            self.spawn_worker(agent_id);
                            self.show_spawn_dialog = false;
                            self.selected_agent_id = None;
                            self.worker_args.clear();
                        }
                    }
                    
                    if ui.button("Cancel").clicked() {
                        self.show_spawn_dialog = false;
                        self.selected_agent_id = None;
                        self.worker_args.clear();
                    }
                });
            });
    }

    fn spawn_worker(&mut self, agent_id: i64) {
        // Get agent details
        let agent = match self.available_agents.iter().find(|a| a.id == Some(agent_id)) {
            Some(agent) => agent,
            None => {
                self.error_message = Some("Agent not found".to_string());
                return;
            }
        };

        // Build command to spawn worker process
        let mut cmd = Command::new("cargo");
        cmd.arg("run")
            .arg("--bin")
            .arg("worker")
            .arg("--")
            .arg("--agent-id")
            .arg(agent_id.to_string());

        // Add any additional arguments
        if !self.worker_args.is_empty() {
            for arg in self.worker_args.split_whitespace() {
                cmd.arg(arg);
            }
        }

        // Spawn the process
        match cmd.spawn() {
            Ok(child) => {
                let worker_id = self.next_worker_id;
                self.next_worker_id += 1;

                let worker_info = WorkerInfo {
                    id: worker_id,
                    agent_name: agent.name.clone(),
                    agent_id,
                    process: Arc::new(Mutex::new(child)),
                    status: WorkerStatus::Running,
                    started_at: Instant::now(),
                    last_heartbeat: Instant::now(),
                    logs: vec![format!("Worker spawned at {}", chrono::Local::now())],
                };

                self.workers.insert(worker_id, worker_info);
                self.selected_worker = Some(worker_id);
            }
            Err(e) => {
                self.error_message = Some(format!("Failed to spawn worker: {}", e));
            }
        }
    }

    fn stop_worker(&mut self, worker_id: u64) {
        if let Some(worker) = self.workers.get_mut(&worker_id) {
            if let Ok(mut process) = worker.process.lock() {
                match process.kill() {
                    Ok(_) => {
                        worker.status = WorkerStatus::Stopped;
                        worker.logs.push(format!("Worker stopped at {}", chrono::Local::now()));
                    }
                    Err(e) => {
                        self.error_message = Some(format!("Failed to stop worker: {}", e));
                    }
                }
            }
        }
    }

    fn restart_worker(&mut self, worker_id: u64) {
        if let Some(worker) = self.workers.get(&worker_id) {
            let agent_id = worker.agent_id;
            self.stop_worker(worker_id);
            self.spawn_worker(agent_id);
        }
    }

    fn refresh_worker_status(&mut self) {
        let worker_ids: Vec<u64> = self.workers.keys().copied().collect();
        
        for worker_id in worker_ids {
            if let Some(worker) = self.workers.get_mut(&worker_id) {
                if let Ok(mut process) = worker.process.lock() {
                    match process.try_wait() {
                        Ok(Some(status)) => {
                            if status.success() {
                                worker.status = WorkerStatus::Stopped;
                            } else {
                                worker.status = WorkerStatus::Crashed;
                            }
                        }
                        Ok(None) => {
                            // Process is still running
                            worker.status = WorkerStatus::Running;
                        }
                        Err(_) => {
                            worker.status = WorkerStatus::Unknown;
                        }
                    }
                }
            }
        }
    }
}