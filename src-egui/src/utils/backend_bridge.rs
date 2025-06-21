// Direct backend integration without going through HTTP/Tauri
use crate::models::TauriAgent;
use anyhow::Result;
use rusqlite::{params, Connection};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

pub struct BackendBridge {
    db_path: PathBuf,
    db: Arc<Mutex<Connection>>,
}

impl BackendBridge {
    pub fn new() -> Result<Self> {
        let db_path = dirs::home_dir()
            .ok_or_else(|| anyhow::anyhow!("Failed to get home directory"))?
            .join(".claudia")
            .join("agents.db");
        
        // Create directory if it doesn't exist
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        
        let conn = Connection::open(&db_path)?;
        
        // Initialize database schema
        conn.execute(
            "CREATE TABLE IF NOT EXISTS agents (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                icon TEXT NOT NULL DEFAULT '🤖',
                system_prompt TEXT NOT NULL,
                default_task TEXT,
                model TEXT NOT NULL DEFAULT 'claude-3-opus-20240229',
                sandbox_enabled BOOLEAN DEFAULT 0,
                enable_file_read BOOLEAN DEFAULT 1,
                enable_file_write BOOLEAN DEFAULT 0,
                enable_network BOOLEAN DEFAULT 0,
                enable_system_commands BOOLEAN DEFAULT 0,
                custom_instructions TEXT,
                sandbox_profile_id INTEGER,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            )",
            [],
        )?;
        
        Ok(Self {
            db_path,
            db: Arc::new(Mutex::new(conn)),
        })
    }
    
    pub fn list_agents(&self) -> Result<Vec<TauriAgent>> {
        let conn = self.db.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, name, icon, system_prompt, default_task, model, 
                    sandbox_enabled, enable_file_read, enable_file_write, 
                    enable_network, enable_system_commands, custom_instructions,
                    sandbox_profile_id, created_at, updated_at
             FROM agents ORDER BY created_at DESC"
        )?;
        
        let agents = stmt.query_map([], |row| {
            Ok(TauriAgent {
                id: Some(row.get(0)?),
                name: row.get(1)?,
                icon: row.get(2)?,
                system_prompt: row.get(3)?,
                default_task: row.get(4)?,
                model: row.get(5)?,
                sandbox_enabled: row.get(6)?,
                enable_file_read: row.get(7)?,
                enable_file_write: row.get(8)?,
                enable_network: row.get(9)?,
                enable_system_commands: row.get(10)?,
                custom_instructions: row.get(11)?,
                sandbox_profile_id: row.get(12)?,
                created_at: row.get(13)?,
                updated_at: row.get(14)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
        
        Ok(agents)
    }
    
    pub fn create_agent(&self, agent: &TauriAgent) -> Result<i64> {
        let conn = self.db.lock().unwrap();
        conn.execute(
            "INSERT INTO agents (name, icon, system_prompt, default_task, model,
                                sandbox_enabled, enable_file_read, enable_file_write,
                                enable_network, enable_system_commands, custom_instructions,
                                sandbox_profile_id)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![
                agent.name,
                agent.icon,
                agent.system_prompt,
                agent.default_task,
                agent.model,
                agent.sandbox_enabled,
                agent.enable_file_read,
                agent.enable_file_write,
                agent.enable_network,
                agent.enable_system_commands,
                agent.custom_instructions,
                agent.sandbox_profile_id,
            ],
        )?;
        
        Ok(conn.last_insert_rowid())
    }
    
    pub fn update_agent(&self, agent: &TauriAgent) -> Result<()> {
        if let Some(id) = agent.id {
            let conn = self.db.lock().unwrap();
            conn.execute(
                "UPDATE agents SET name = ?1, icon = ?2, system_prompt = ?3,
                                  default_task = ?4, model = ?5, sandbox_enabled = ?6,
                                  enable_file_read = ?7, enable_file_write = ?8,
                                  enable_network = ?9, enable_system_commands = ?10,
                                  custom_instructions = ?11, sandbox_profile_id = ?12,
                                  updated_at = CURRENT_TIMESTAMP
                 WHERE id = ?13",
                params![
                    agent.name,
                    agent.icon,
                    agent.system_prompt,
                    agent.default_task,
                    agent.model,
                    agent.sandbox_enabled,
                    agent.enable_file_read,
                    agent.enable_file_write,
                    agent.enable_network,
                    agent.enable_system_commands,
                    agent.custom_instructions,
                    agent.sandbox_profile_id,
                    id,
                ],
            )?;
        }
        Ok(())
    }
    
    pub fn delete_agent(&self, id: i64) -> Result<()> {
        let conn = self.db.lock().unwrap();
        conn.execute("DELETE FROM agents WHERE id = ?1", params![id])?;
        Ok(())
    }
    
    pub fn get_agent(&self, id: i64) -> Result<TauriAgent> {
        let conn = self.db.lock().unwrap();
        let agent = conn.query_row(
            "SELECT id, name, icon, system_prompt, default_task, model, 
                    sandbox_enabled, enable_file_read, enable_file_write, 
                    enable_network, enable_system_commands, custom_instructions,
                    sandbox_profile_id, created_at, updated_at
             FROM agents WHERE id = ?1",
            params![id],
            |row| {
                Ok(TauriAgent {
                    id: Some(row.get(0)?),
                    name: row.get(1)?,
                    icon: row.get(2)?,
                    system_prompt: row.get(3)?,
                    default_task: row.get(4)?,
                    model: row.get(5)?,
                    sandbox_enabled: row.get(6)?,
                    enable_file_read: row.get(7)?,
                    enable_file_write: row.get(8)?,
                    enable_network: row.get(9)?,
                    enable_system_commands: row.get(10)?,
                    custom_instructions: row.get(11)?,
                    sandbox_profile_id: row.get(12)?,
                    created_at: row.get(13)?,
                    updated_at: row.get(14)?,
                })
            }
        )?;
        
        Ok(agent)
    }
}