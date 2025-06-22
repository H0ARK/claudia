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
        
        // Create workflows table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS workflows (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                description TEXT,
                nodes TEXT NOT NULL, -- JSON
                edges TEXT NOT NULL, -- JSON
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            )",
            [],
        )?;
        
        // Create orchestrations table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS orchestrations (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                workflow_id INTEGER,
                status TEXT NOT NULL DEFAULT 'pending',
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                started_at TIMESTAMP,
                completed_at TIMESTAMP,
                FOREIGN KEY (workflow_id) REFERENCES workflows(id)
            )",
            [],
        )?;
        
        // Create tasks table for orchestration tasks
        conn.execute(
            "CREATE TABLE IF NOT EXISTS tasks (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                orchestration_id INTEGER NOT NULL,
                name TEXT NOT NULL,
                description TEXT,
                agent_id INTEGER,
                status TEXT NOT NULL DEFAULT 'pending',
                input_data TEXT, -- JSON
                output_data TEXT, -- JSON
                error TEXT,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                started_at TIMESTAMP,
                completed_at TIMESTAMP,
                FOREIGN KEY (orchestration_id) REFERENCES orchestrations(id),
                FOREIGN KEY (agent_id) REFERENCES agents(id)
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
    
    // Workflow methods
    pub fn save_workflow(&self, name: &str, description: &str, nodes: &str, edges: &str) -> Result<i64> {
        let conn = self.db.lock().unwrap();
        conn.execute(
            "INSERT INTO workflows (name, description, nodes, edges) VALUES (?1, ?2, ?3, ?4)",
            params![name, description, nodes, edges],
        )?;
        Ok(conn.last_insert_rowid())
    }
    
    pub fn update_workflow(&self, id: i64, name: &str, description: &str, nodes: &str, edges: &str) -> Result<()> {
        let conn = self.db.lock().unwrap();
        conn.execute(
            "UPDATE workflows SET name = ?1, description = ?2, nodes = ?3, edges = ?4, updated_at = CURRENT_TIMESTAMP WHERE id = ?5",
            params![name, description, nodes, edges, id],
        )?;
        Ok(())
    }
    
    pub fn list_workflows(&self) -> Result<Vec<(i64, String, String, String, String)>> {
        let conn = self.db.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, name, description, nodes, edges FROM workflows ORDER BY created_at DESC"
        )?;
        
        let workflows = stmt.query_map([], |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2).unwrap_or_default(),
                row.get(3)?,
                row.get(4)?,
            ))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
        
        Ok(workflows)
    }
    
    pub fn get_workflow(&self, id: i64) -> Result<(String, String, String, String)> {
        let conn = self.db.lock().unwrap();
        conn.query_row(
            "SELECT name, description, nodes, edges FROM workflows WHERE id = ?1",
            params![id],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1).unwrap_or_default(),
                    row.get(2)?,
                    row.get(3)?,
                ))
            }
        ).map_err(Into::into)
    }
    
    pub fn delete_workflow(&self, id: i64) -> Result<()> {
        let conn = self.db.lock().unwrap();
        conn.execute("DELETE FROM workflows WHERE id = ?1", params![id])?;
        Ok(())
    }
    
    // Orchestration methods
    pub fn create_orchestration(&self, name: &str, workflow_id: Option<i64>) -> Result<i64> {
        let conn = self.db.lock().unwrap();
        conn.execute(
            "INSERT INTO orchestrations (name, workflow_id) VALUES (?1, ?2)",
            params![name, workflow_id],
        )?;
        Ok(conn.last_insert_rowid())
    }
    
    pub fn start_orchestration(&self, id: i64) -> Result<()> {
        let conn = self.db.lock().unwrap();
        conn.execute(
            "UPDATE orchestrations SET status = 'running', started_at = CURRENT_TIMESTAMP WHERE id = ?1",
            params![id],
        )?;
        Ok(())
    }
    
    pub fn complete_orchestration(&self, id: i64, success: bool) -> Result<()> {
        let conn = self.db.lock().unwrap();
        let status = if success { "completed" } else { "failed" };
        conn.execute(
            "UPDATE orchestrations SET status = ?1, completed_at = CURRENT_TIMESTAMP WHERE id = ?2",
            params![status, id],
        )?;
        Ok(())
    }
    
    // Task methods
    pub fn create_task(&self, orchestration_id: i64, name: &str, description: &str, agent_id: Option<i64>) -> Result<i64> {
        let conn = self.db.lock().unwrap();
        conn.execute(
            "INSERT INTO tasks (orchestration_id, name, description, agent_id) VALUES (?1, ?2, ?3, ?4)",
            params![orchestration_id, name, description, agent_id],
        )?;
        Ok(conn.last_insert_rowid())
    }
    
    pub fn update_task_status(&self, id: i64, status: &str, output: Option<&str>, error: Option<&str>) -> Result<()> {
        let conn = self.db.lock().unwrap();
        let timestamp_field = match status {
            "running" => "started_at",
            "completed" | "failed" => "completed_at",
            _ => return Ok(()),
        };
        
        conn.execute(
            &format!("UPDATE tasks SET status = ?1, output_data = ?2, error = ?3, {} = CURRENT_TIMESTAMP WHERE id = ?4", timestamp_field),
            params![status, output, error, id],
        )?;
        Ok(())
    }
}