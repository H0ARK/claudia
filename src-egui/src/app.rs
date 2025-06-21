use crate::orchestration::OrchestrationEngine;
use crate::ui::{AgentManagerView, OrchestratorView, WorkflowEditorView};
use crate::utils::ipc::IpcClient;
use eframe::CreationContext;
use egui::Context;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AppView {
    Orchestrator,
    AgentManager,
    WorkflowEditor,
    Metrics,
    Settings,
}

pub struct ClaudiaOrchestratorApp {
    current_view: AppView,
    orchestrator_view: OrchestratorView,
    agent_manager_view: AgentManagerView,
    workflow_editor_view: WorkflowEditorView,
    orchestration_engine: OrchestrationEngine,
    ipc_client: IpcClient,
    dark_mode: bool,
}

impl ClaudiaOrchestratorApp {
    pub fn new(cc: &CreationContext) -> Self {
        // Configure fonts
        configure_fonts(&cc.egui_ctx);

        // Configure style
        configure_style(&cc.egui_ctx);

        // Initialize IPC client for backend communication
        let ipc_client = IpcClient::new("http://localhost:1420"); // Tauri default port

        // Initialize orchestration engine
        let orchestration_engine = OrchestrationEngine::new();

        // Initialize views
        let orchestrator_view = OrchestratorView::new();
        let agent_manager_view = AgentManagerView::new();
        let workflow_editor_view = WorkflowEditorView::new();

        Self {
            current_view: AppView::Orchestrator,
            orchestrator_view,
            agent_manager_view,
            workflow_editor_view,
            orchestration_engine,
            ipc_client,
            dark_mode: true,
        }
    }
}

impl eframe::App for ClaudiaOrchestratorApp {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        // Top panel with navigation
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("🤖 Claudia Orchestrator");
                
                ui.separator();
                
                ui.selectable_value(&mut self.current_view, AppView::Orchestrator, "📊 Orchestrator");
                ui.selectable_value(&mut self.current_view, AppView::AgentManager, "🤖 Agents");
                ui.selectable_value(&mut self.current_view, AppView::WorkflowEditor, "🔀 Workflows");
                ui.selectable_value(&mut self.current_view, AppView::Metrics, "📈 Metrics");
                
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("⚙").clicked() {
                        self.current_view = AppView::Settings;
                    }
                    
                    let theme_icon = if self.dark_mode { "🌙" } else { "☀" };
                    if ui.button(theme_icon).clicked() {
                        self.dark_mode = !self.dark_mode;
                        let theme = if self.dark_mode {
                            egui::Visuals::dark()
                        } else {
                            egui::Visuals::light()
                        };
                        ctx.set_visuals(theme);
                    }
                });
            });
        });

        // Main content
        match self.current_view {
            AppView::Orchestrator => {
                self.orchestrator_view.show(ctx, &mut self.orchestration_engine);
            }
            AppView::AgentManager => {
                self.agent_manager_view.show(ctx, &mut self.ipc_client);
            }
            AppView::WorkflowEditor => {
                self.workflow_editor_view.show(ctx);
            }
            AppView::Metrics => {
                egui::CentralPanel::default().show(ctx, |ui| {
                    ui.heading("Metrics Dashboard");
                    ui.label("Coming soon...");
                });
            }
            AppView::Settings => {
                egui::CentralPanel::default().show(ctx, |ui| {
                    ui.heading("Settings");
                    ui.label("Coming soon...");
                });
            }
        }
    }
}

fn configure_fonts(ctx: &Context) {
    let fonts = egui::FontDefinitions::default();
    
    // Add custom fonts if needed
    // fonts.font_data.insert(
    //     "custom_font".to_owned(),
    //     egui::FontData::from_static(include_bytes!("../assets/font.ttf")),
    // );
    
    ctx.set_fonts(fonts);
}

fn configure_style(ctx: &Context) {
    let mut style = (*ctx.style()).clone();
    
    // Customize style
    style.spacing.item_spacing = egui::vec2(8.0, 4.0);
    style.spacing.button_padding = egui::vec2(8.0, 4.0);
    style.spacing.window_margin = 8.0.into();
    
    ctx.set_style(style);
}