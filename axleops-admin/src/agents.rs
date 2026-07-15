use crate::models::{AgentInfo, RegisterAgentRequest, UpdateAgentRequest};
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum RegistryError {
    #[error("agent not found")]
    NotFound,
    #[error("agent name already exists: {0}")]
    DuplicateName(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("db error: {0}")]
    Db(#[from] rusqlite::Error),
    #[error("{0}")]
    Other(String),
}

pub struct AgentRegistry {
    db_path: PathBuf,
    conn: Mutex<Connection>,
}

impl AgentRegistry {
    pub fn load(data_dir: &Path) -> anyhow::Result<Self> {
        fs::create_dir_all(data_dir)?;
        let db_path = data_dir.join("axleops.db");
        let conn = Connection::open(&db_path)?;
        conn.execute_batch(
            "
            PRAGMA foreign_keys = ON;
            CREATE TABLE IF NOT EXISTS agents (
                id TEXT PRIMARY KEY NOT NULL,
                name TEXT NOT NULL UNIQUE,
                base_url TEXT NOT NULL,
                token TEXT NOT NULL,
                tags TEXT NOT NULL DEFAULT '[]',
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );
            ",
        )?;

        let registry = Self {
            db_path: db_path.clone(),
            conn: Mutex::new(conn),
        };
        registry.migrate_from_json_if_needed(data_dir)?;
        tracing::info!(db = %db_path.display(), "sqlite ready");
        Ok(registry)
    }

    fn migrate_from_json_if_needed(&self, data_dir: &Path) -> anyhow::Result<()> {
        let json_path = data_dir.join("agents.json");
        if !json_path.exists() {
            return Ok(());
        }

        let count: i64 = {
            let conn = self
                .conn
                .lock()
                .map_err(|_| anyhow::anyhow!("db lock poisoned"))?;
            conn.query_row("SELECT COUNT(*) FROM agents", [], |row| row.get(0))?
        };
        if count > 0 {
            tracing::info!("sqlite already has agents; leaving agents.json in place");
            return Ok(());
        }

        let text = fs::read_to_string(&json_path)?;
        let list: Vec<AgentInfo> = serde_json::from_str(&text).unwrap_or_default();
        if list.is_empty() {
            return Ok(());
        }

        {
            let conn = self
                .conn
                .lock()
                .map_err(|_| anyhow::anyhow!("db lock poisoned"))?;
            let tx = conn.unchecked_transaction()?;
            for agent in &list {
                tx.execute(
                    "INSERT INTO agents (id, name, base_url, token, tags, created_at, updated_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                    params![
                        agent.id,
                        agent.name,
                        agent.base_url,
                        agent.token,
                    tags_to_json(&agent.tags).map_err(|e| anyhow::anyhow!(e))?,
                    agent.created_at.to_rfc3339(),
                    agent.updated_at.to_rfc3339(),
                ],
            )?;
            }
            tx.commit()?;
        }

        let bak = data_dir.join("agents.json.bak");
        fs::rename(&json_path, &bak)?;
        tracing::info!(
            count = list.len(),
            bak = %bak.display(),
            "migrated agents.json into sqlite"
        );
        Ok(())
    }

    fn with_conn<T>(
        &self,
        f: impl FnOnce(&Connection) -> Result<T, RegistryError>,
    ) -> Result<T, RegistryError> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| RegistryError::Other("db lock poisoned".into()))?;
        f(&conn)
    }

    pub fn list(&self) -> Result<Vec<AgentInfo>, RegistryError> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, name, base_url, token, tags, created_at, updated_at
                 FROM agents ORDER BY name ASC",
            )?;
            let rows = stmt.query_map([], row_to_agent)?;
            let mut out = Vec::new();
            for row in rows {
                out.push(row?);
            }
            Ok(out)
        })
    }

    pub fn get(&self, id: &str) -> Result<AgentInfo, RegistryError> {
        self.with_conn(|conn| {
            conn.query_row(
                "SELECT id, name, base_url, token, tags, created_at, updated_at
                 FROM agents WHERE id = ?1",
                params![id],
                row_to_agent,
            )
            .optional()?
            .ok_or(RegistryError::NotFound)
        })
    }

    pub fn register(&self, req: RegisterAgentRequest) -> Result<AgentInfo, RegistryError> {
        let name = req.name.trim().to_string();
        let base_url = req.base_url.trim().trim_end_matches('/').to_string();
        if name.is_empty() {
            return Err(RegistryError::Other("name is required".into()));
        }
        if base_url.is_empty() {
            return Err(RegistryError::Other("base_url is required".into()));
        }
        if req.token.trim().is_empty() {
            return Err(RegistryError::Other("token is required".into()));
        }

        let now = Utc::now();
        let agent = AgentInfo {
            id: Uuid::new_v4().to_string(),
            name,
            base_url,
            token: req.token,
            tags: req.tags,
            created_at: now,
            updated_at: now,
        };

        self.with_conn(|conn| {
            match conn.execute(
                "INSERT INTO agents (id, name, base_url, token, tags, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    agent.id,
                    agent.name,
                    agent.base_url,
                    agent.token,
                    tags_to_json(&agent.tags)?,
                    agent.created_at.to_rfc3339(),
                    agent.updated_at.to_rfc3339(),
                ],
            ) {
                Ok(_) => Ok(agent.clone()),
                Err(rusqlite::Error::SqliteFailure(err, _))
                    if err.code == rusqlite::ErrorCode::ConstraintViolation =>
                {
                    Err(RegistryError::DuplicateName(agent.name.clone()))
                }
                Err(e) => Err(RegistryError::Db(e)),
            }
        })
    }

    pub fn update(&self, id: &str, req: UpdateAgentRequest) -> Result<AgentInfo, RegistryError> {
        let mut agent = self.get(id)?;

        if let Some(ref name) = req.name {
            let name = name.trim();
            if name.is_empty() {
                return Err(RegistryError::Other("name cannot be empty".into()));
            }
            agent.name = name.to_string();
        }
        if let Some(ref url) = req.base_url {
            let url = url.trim().trim_end_matches('/');
            if url.is_empty() {
                return Err(RegistryError::Other("base_url cannot be empty".into()));
            }
            agent.base_url = url.to_string();
        }
        if let Some(ref token) = req.token {
            if token.trim().is_empty() {
                return Err(RegistryError::Other("token cannot be empty".into()));
            }
            agent.token = token.clone();
        }
        if let Some(tags) = req.tags {
            agent.tags = tags;
        }
        agent.updated_at = Utc::now();

        self.with_conn(|conn| {
            match conn.execute(
                "UPDATE agents
                 SET name = ?1, base_url = ?2, token = ?3, tags = ?4, updated_at = ?5
                 WHERE id = ?6",
                params![
                    agent.name,
                    agent.base_url,
                    agent.token,
                    tags_to_json(&agent.tags)?,
                    agent.updated_at.to_rfc3339(),
                    id,
                ],
            ) {
                Ok(0) => Err(RegistryError::NotFound),
                Ok(_) => Ok(agent.clone()),
                Err(rusqlite::Error::SqliteFailure(err, _))
                    if err.code == rusqlite::ErrorCode::ConstraintViolation =>
                {
                    Err(RegistryError::DuplicateName(agent.name.clone()))
                }
                Err(e) => Err(RegistryError::Db(e)),
            }
        })
    }

    pub fn remove(&self, id: &str) -> Result<AgentInfo, RegistryError> {
        let agent = self.get(id)?;
        self.with_conn(|conn| {
            let n = conn.execute("DELETE FROM agents WHERE id = ?1", params![id])?;
            if n == 0 {
                return Err(RegistryError::NotFound);
            }
            Ok(agent)
        })
    }

    pub fn db_path(&self) -> &Path {
        &self.db_path
    }
}

fn tags_to_json(tags: &[String]) -> Result<String, RegistryError> {
    serde_json::to_string(tags).map_err(|e| RegistryError::Other(e.to_string()))
}

fn tags_from_json(s: &str) -> Result<Vec<String>, RegistryError> {
    serde_json::from_str(s).map_err(|e| RegistryError::Other(e.to_string()))
}

fn parse_rfc3339(s: &str) -> Result<DateTime<Utc>, RegistryError> {
    DateTime::parse_from_rfc3339(s)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|e| RegistryError::Other(format!("invalid datetime: {e}")))
}

fn row_to_agent(row: &rusqlite::Row<'_>) -> rusqlite::Result<AgentInfo> {
    let tags_json: String = row.get(4)?;
    let created_at: String = row.get(5)?;
    let updated_at: String = row.get(6)?;
    let tags = tags_from_json(&tags_json).map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(4, rusqlite::types::Type::Text, Box::new(e))
    })?;
    let created_at = parse_rfc3339(&created_at).map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(5, rusqlite::types::Type::Text, Box::new(e))
    })?;
    let updated_at = parse_rfc3339(&updated_at).map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(6, rusqlite::types::Type::Text, Box::new(e))
    })?;
    Ok(AgentInfo {
        id: row.get(0)?,
        name: row.get(1)?,
        base_url: row.get(2)?,
        token: row.get(3)?,
        tags,
        created_at,
        updated_at,
    })
}
