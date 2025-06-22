use anyhow::Result;
use std::process::{Command, Stdio};
use tokio::signal;
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logger
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    
    println!("🚀 Starting Claudia Orchestrator...");
    println!("📱 eGUI Frontend will start first");
    println!("⚡ Tauri Backend will start after a short delay");
    println!("Press Ctrl+C to stop both applications");
    println!("{}", "─".repeat(60));
    
    // Start both applications concurrently
    let egui_handle = tokio::spawn(start_egui());
    let tauri_handle = tokio::spawn(start_tauri());
    
    // Set up graceful shutdown
    tokio::select! {
        result = egui_handle => {
            match result {
                Ok(Ok(())) => println!("✅ eGUI frontend completed successfully"),
                Ok(Err(e)) => eprintln!("❌ eGUI frontend failed: {}", e),
                Err(e) => eprintln!("💥 eGUI frontend task panicked: {}", e),
            }
        }
        result = tauri_handle => {
            match result {
                Ok(Ok(())) => println!("✅ Tauri backend completed successfully"),
                Ok(Err(e)) => eprintln!("❌ Tauri backend failed: {}", e),
                Err(e) => eprintln!("💥 Tauri backend task panicked: {}", e),
            }
        }
        _ = signal::ctrl_c() => {
            println!("\n🛑 Received Ctrl+C, shutting down gracefully...");
        }
    }
    
    println!("👋 Claudia Orchestrator stopped");
    Ok(())
}

async fn start_egui() -> Result<()> {
    println!("📱 [eGUI] Starting frontend...");
    
    // Give a small delay to let any previous processes clean up
    sleep(Duration::from_millis(500)).await;
    
    let mut child = Command::new("cargo")
        .args(&["run", "--manifest-path", "src-egui/Cargo.toml"])
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()?;
    
    println!("✅ [eGUI] Frontend started (PID: {})", child.id());
    
    let status = child.wait()?;
    if status.success() {
        println!("📱 [eGUI] Frontend exited successfully");
        Ok(())
    } else {
        Err(anyhow::anyhow!("eGUI frontend exited with status: {}", status))
    }
}

async fn start_tauri() -> Result<()> {
    println!("⚡ [Tauri] Starting backend...");
    
    // Give a delay to let eGUI start first
    sleep(Duration::from_millis(2000)).await;
    
    let mut child = Command::new("cargo")
        .args(&["run", "--manifest-path", "src-tauri/Cargo.toml"])
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()?;
    
    println!("✅ [Tauri] Backend started (PID: {})", child.id());
    
    let status = child.wait()?;
    if status.success() {
        println!("⚡ [Tauri] Backend exited successfully");
        Ok(())
    } else {
        Err(anyhow::anyhow!("Tauri backend exited with status: {}", status))
    }
} 