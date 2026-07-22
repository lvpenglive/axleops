use crate::models::{ProxyInfo, RegisterProxyRequest, UpdateProxyRequest};
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum ProxyRegistryError {
    #[error("proxy not found")]
    NotFound,
    #[error("proxy name already exists: {0}")]
    DuplicateName(String),
    #[error("db error: {0}")]
    Db(#[from] rusqlite::Error),
    #[error("{0}")]
    Other(String),
}

pub struct ProxyRegistry {
    db_path: PathBuf,
    conn: Mutex<Connection>,
}

impl ProxyRegistry {
    pub fn open(db_path: &Path) -> anyhow::Result<Self> {
        let conn = Connection::open(db_path)?;
        conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS proxies (
                id TEXT PRIMARY KEY NOT NULL,
                name TEXT NOT NULL UNIQUE,
                base_url TEXT NOT NULL,
                token TEXT NOT NULL,
                notes TEXT NOT NULL DEFAULT '',
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );
            ",
        )?;

        Ok(Self {
            db_path: db_path.to_path_buf(),
            conn: Mutex::new(conn),
        })
    }

    fn with_conn<T>(
        &self,
        f: impl FnOnce(&Connection) -> Result<T, ProxyRegistryError>,
    ) -> Result<T, ProxyRegistryError> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| ProxyRegistryError::Other("db lock poisoned".into()))?;
        f(&conn)
    }

    pub fn list(&self) -> Result<Vec<ProxyInfo>, ProxyRegistryError> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, name, base_url, token, notes, created_at, updated_at
                 FROM proxies ORDER BY name ASC",
            )?;
            let rows = stmt.query_map([], row_to_proxy)?;
            let mut out = Vec::new();
            for row in rows {
                out.push(row?);
            }
            Ok(out)
        })
    }

    pub fn get(&self, id: &str) -> Result<ProxyInfo, ProxyRegistryError> {
        self.with_conn(|conn| {
            conn.query_row(
                "SELECT id, name, base_url, token, notes, created_at, updated_at
                 FROM proxies WHERE id = ?1",
                params![id],
                row_to_proxy,
            )
            .optional()?
            .ok_or(ProxyRegistryError::NotFound)
        })
    }

    pub fn register(&self, req: RegisterProxyRequest) -> Result<ProxyInfo, ProxyRegistryError> {
        let name = req.name.trim().to_string();
        let base_url = req.base_url.trim().trim_end_matches('/').to_string();
        if name.is_empty() {
            return Err(ProxyRegistryError::Other("name is required".into()));
        }
        if base_url.is_empty() {
            return Err(ProxyRegistryError::Other("base_url is required".into()));
        }
        let token = req.token.trim().to_string();
        if token.is_empty() {
            return Err(ProxyRegistryError::Other("token is required".into()));
        }

        let now = Utc::now();
        let proxy = ProxyInfo {
            id: Uuid::new_v4().to_string(),
            name,
            base_url,
            token,
            notes: req.notes.unwrap_or_default(),
            created_at: now,
            updated_at: now,
        };

        self.with_conn(|conn| {
            match conn.execute(
                "INSERT INTO proxies (id, name, base_url, token, notes, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    proxy.id,
                    proxy.name,
                    proxy.base_url,
                    proxy.token,
                    proxy.notes,
                    proxy.created_at.to_rfc3339(),
                    proxy.updated_at.to_rfc3339(),
                ],
            ) {
                Ok(_) => Ok(proxy.clone()),
                Err(rusqlite::Error::SqliteFailure(err, _))
                    if err.code == rusqlite::ErrorCode::ConstraintViolation =>
                {
                    Err(ProxyRegistryError::DuplicateName(proxy.name.clone()))
                }
                Err(e) => Err(ProxyRegistryError::Db(e)),
            }
        })
    }

    pub fn update(&self, id: &str, req: UpdateProxyRequest) -> Result<ProxyInfo, ProxyRegistryError> {
        let mut proxy = self.get(id)?;
        if let Some(ref name) = req.name {
            let name = name.trim();
            if name.is_empty() {
                return Err(ProxyRegistryError::Other("name cannot be empty".into()));
            }
            proxy.name = name.to_string();
        }
        if let Some(ref url) = req.base_url {
            let url = url.trim().trim_end_matches('/');
            if url.is_empty() {
                return Err(ProxyRegistryError::Other("base_url cannot be empty".into()));
            }
            proxy.base_url = url.to_string();
        }
        if let Some(ref token) = req.token {
            let token = token.trim().to_string();
            if token.is_empty() {
                return Err(ProxyRegistryError::Other("token cannot be empty".into()));
            }
            proxy.token = token;
        }
        if let Some(notes) = req.notes {
            proxy.notes = notes;
        }
        proxy.updated_at = Utc::now();

        self.with_conn(|conn| {
            match conn.execute(
                "UPDATE proxies
                 SET name = ?1, base_url = ?2, token = ?3, notes = ?4, updated_at = ?5
                 WHERE id = ?6",
                params![
                    proxy.name,
                    proxy.base_url,
                    proxy.token,
                    proxy.notes,
                    proxy.updated_at.to_rfc3339(),
                    id,
                ],
            ) {
                Ok(0) => Err(ProxyRegistryError::NotFound),
                Ok(_) => Ok(proxy.clone()),
                Err(rusqlite::Error::SqliteFailure(err, _))
                    if err.code == rusqlite::ErrorCode::ConstraintViolation =>
                {
                    Err(ProxyRegistryError::DuplicateName(proxy.name.clone()))
                }
                Err(e) => Err(ProxyRegistryError::Db(e)),
            }
        })
    }

    pub fn remove(&self, id: &str) -> Result<ProxyInfo, ProxyRegistryError> {
        let proxy = self.get(id)?;
        self.with_conn(|conn| {
            // Clear agent links; do not delete agents.
            let _ = conn.execute(
                "UPDATE agents SET proxy_id = NULL WHERE proxy_id = ?1",
                params![id],
            );
            let n = conn.execute("DELETE FROM proxies WHERE id = ?1", params![id])?;
            if n == 0 {
                return Err(ProxyRegistryError::NotFound);
            }
            Ok(proxy)
        })
    }

    pub fn db_path(&self) -> &Path {
        &self.db_path
    }
}

fn parse_rfc3339(s: &str) -> Result<DateTime<Utc>, ProxyRegistryError> {
    DateTime::parse_from_rfc3339(s)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|e| ProxyRegistryError::Other(format!("invalid datetime: {e}")))
}

fn row_to_proxy(row: &rusqlite::Row<'_>) -> rusqlite::Result<ProxyInfo> {
    let created_at: String = row.get(5)?;
    let updated_at: String = row.get(6)?;
    let created_at = parse_rfc3339(&created_at).map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(5, rusqlite::types::Type::Text, Box::new(e))
    })?;
    let updated_at = parse_rfc3339(&updated_at).map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(6, rusqlite::types::Type::Text, Box::new(e))
    })?;
    Ok(ProxyInfo {
        id: row.get(0)?,
        name: row.get(1)?,
        base_url: row.get(2)?,
        token: row.get(3)?,
        notes: row.get(4)?,
        created_at,
        updated_at,
    })
}
