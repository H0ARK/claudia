// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/

// Declare modules
pub mod commands;
#[cfg(unix)]
pub mod sandbox;
#[cfg(not(unix))]
pub mod sandbox {
    // Stub implementations for Windows
    pub mod profile {
        use serde::{Deserialize, Serialize};
        
        #[derive(Debug, Clone, Serialize, Deserialize)]
        pub struct ProfileBuilder;
        
        #[derive(Debug, Clone, Serialize, Deserialize)]
        pub struct SandboxRule;
        
        impl ProfileBuilder {
            pub fn new() -> Self { Self }
            pub fn build(self) -> Result<Profile, String> { 
                Ok(Profile)
            }
        }
        
        #[derive(Debug, Clone)]
        pub struct Profile;
    }
    
    pub mod executor {
        use std::path::PathBuf;
        
        #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
        pub struct SerializedProfile;
        
        #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
        pub struct SerializedOperation;
        
        pub struct SandboxExecutor;
        
        impl SandboxExecutor {
            pub fn activate_sandbox_in_child() -> Result<(), anyhow::Error> {
                Ok(()) // No-op on Windows
            }
        }
        
        pub fn should_activate_sandbox() -> bool {
            false // Always false on Windows
        }
    }
    
    pub mod platform {
        use serde::{Deserialize, Serialize};
        
        #[derive(Debug, Clone, Serialize, Deserialize)]
        pub struct PlatformCapabilities {
            pub platform: String,
            pub sandboxing_available: bool,
            pub sandbox_methods: Vec<String>,
        }
        
        pub fn get_platform_capabilities() -> PlatformCapabilities {
            PlatformCapabilities {
                platform: "windows".to_string(),
                sandboxing_available: false,
                sandbox_methods: vec![],
            }
        }
    }
    
    pub mod defaults {
        pub fn create_default_profiles() {}
    }
    
    pub use profile::{ProfileBuilder, SandboxRule};
    pub use executor::{SandboxExecutor, should_activate_sandbox};
    pub use platform::{PlatformCapabilities, get_platform_capabilities};
    pub use defaults::create_default_profiles;
}
pub mod checkpoint;
pub mod process;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
