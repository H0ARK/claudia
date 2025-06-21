#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod models;
mod orchestration;
mod ui;
mod utils;

use app::ClaudiaOrchestratorApp;

fn main() -> eframe::Result<()> {
    // Initialize logger
    env_logger::init();

    // Configure native options
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Claudia Orchestrator")
            .with_inner_size([1400.0, 900.0])
            .with_min_inner_size([800.0, 600.0])
            .with_icon(load_icon()),
        ..Default::default()
    };

    // Run the app
    eframe::run_native(
        "Claudia Orchestrator",
        native_options,
        Box::new(|cc| Ok(Box::new(ClaudiaOrchestratorApp::new(cc)))),
    )
}

fn load_icon() -> egui::IconData {
    // TODO: Load actual icon
    egui::IconData::default()
}