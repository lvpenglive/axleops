use chrono::Utc;
use rusqlite::{params, Connection};
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum AuditError {
    #[error("db error: {0}")]
    Db(#[from] rusqlite::Error),
    #[error("{0}")]
    Other(String),
}

#[derive(Debug, Clone, Serialize)]
pub struct AuditEntry {
    pub id: String,
    pub user_id: Option<String>,
    pub username: String,
    pub action: String,
    pub resource_type: String,
    pub resource_id: String,
    pub detail: String,
    pub created_at: String,
}

pub struct AuditStore {
    #[allow(dead_code)]
    db_path: PathBuf,
    conn: Mutex<Connection>,
}

impl AuditStore {
    pub fn open(db_path: &Path) -> anyhow::Result<Self> {
        let conn = Connection::open(db_path)?;
        conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS audit_logs (
                id TEXT PRIMARY KEY NOT NULL,
                user_id TEXT,
                username TEXT NOT NULL,
                action TEXT NOT NULL,
                resource_type TEXT NOT NULL DEFAULT '',
                resource_id TEXT NOT NULL DEFAULT '',
                detail TEXT NOT NULL DEFAULT '',
                created_at TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_audit_created ON audit_logs(created_at DESC);
            ",
        )?;
        Ok(Self {
            db_path: db_path.to_path_buf(),
            conn: Mutex::new(conn),
        })
    }

    pub fn append(
        &self,
        user_id: Option<&str>,
        username: &str,
        action: &str,
        resource_type: &str,
        resource_id: &str,
        detail: impl Into<String>,
    ) -> Result<(), AuditError> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| AuditError::Other("db lock poisoned".into()))?;
        conn.execute(
            "INSERT INTO audit_logs (id, user_id, username, action, resource_type, resource_id, detail, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                Uuid::new_v4().to_string(),
                user_id,
                username,
                action,
                resource_type,
                resource_id,
                detail.into(),
                Utc::now().to_rfc3339(),
            ],
        )?;
        Ok(())
    }

    pub fn list(&self, limit: usize) -> Result<Vec<AuditEntry>, AuditError> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| AuditError::Other("db lock poisoned".into()))?;
        let lim = limit.clamp(1, 500) as i64;
        let mut stmt = conn.prepare(
            "SELECT id, user_id, username, action, resource_type, resource_id, detail, created_at
             FROM audit_logs ORDER BY created_at DESC LIMIT ?1",
        )?;
        let rows = stmt.query_map(params![lim], |row| {
            Ok(AuditEntry {
                id: row.get(0)?,
                user_id: row.get(1)?,
                username: row.get(2)?,
                action: row.get(3)?,
                resource_type: row.get(4)?,
                resource_id: row.get(5)?,
                detail: row.get(6)?,
                created_at: row.get(7)?,
            })
        })?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }
}
