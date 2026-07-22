use crate::config::{Config, Upstream, UpstreamAgent};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::RwLock;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum StoreError {
    #[error("upstream not found")]
    NotFound,
    #[error("upstream id already exists: {0}")]
    Duplicate(String),
    #[error("{0}")]
    Other(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpstreamRecord {
    pub id: String,
    pub name: String,
    pub base_url: String,
    pub token: String,
}

impl UpstreamRecord {
    pub fn to_upstream(&self) -> Upstream {
        Upstream {
            id: self.id.clone(),
            name: self.name.clone(),
            base_url: self.base_url.trim().trim_end_matches('/').to_string(),
            token: self.token.clone(),
        }
    }

    pub fn from_agent(a: &UpstreamAgent) -> Self {
        let name = a
            .name
            .clone()
            .unwrap_or_else(|| a.id.clone())
            .trim()
            .to_string();
        Self {
            id: a.id.trim().to_string(),
            name: if name.is_empty() {
                a.id.trim().to_string()
            } else {
                name
            },
            base_url: a.base_url.trim().trim_end_matches('/').to_string(),
            token: a.token.clone(),
        }
    }

    pub fn validate(&self) -> Result<(), StoreError> {
        if self.id.trim().is_empty() {
            return Err(StoreError::Other("id is required".into()));
        }
        if self.id.contains('/') || self.id.contains('\\') || self.id.contains(' ') {
            return Err(StoreError::Other(
                "id cannot contain spaces or path separators".into(),
            ));
        }
        if self.base_url.trim().is_empty() {
            return Err(StoreError::Other("base_url is required".into()));
        }
        if self.token.trim().is_empty() {
            return Err(StoreError::Other("token is required".into()));
        }
        if self.name.trim().is_empty() {
            return Err(StoreError::Other("name is required".into()));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpsertUpstreamRequest {
    pub id: String,
    #[serde(default)]
    pub name: Option<String>,
    pub base_url: String,
    pub token: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpdateUpstreamRequest {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub base_url: Option<String>,
    #[serde(default)]
    pub token: Option<String>,
}

pub struct UpstreamStore {
    path: PathBuf,
    inner: RwLock<HashMap<String, UpstreamRecord>>,
}

impl UpstreamStore {
    pub fn load(data_dir: &Path, config: &Config) -> anyhow::Result<Self> {
        fs::create_dir_all(data_dir)?;
        let path = data_dir.join("upstreams.json");
        let map = if path.exists() {
            let text = fs::read_to_string(&path)?;
            let list: Vec<UpstreamRecord> = serde_json::from_str(&text).unwrap_or_default();
            let mut m = HashMap::new();
            for rec in list {
                m.insert(rec.id.clone(), rec);
            }
            tracing::info!(
                path = %path.display(),
                count = m.len(),
                "loaded upstreams from disk"
            );
            m
        } else {
            let mut m = HashMap::new();
            for a in &config.agents {
                let rec = UpstreamRecord::from_agent(a);
                m.insert(rec.id.clone(), rec);
            }
            tracing::info!(
                count = m.len(),
                "seeded upstreams from config.toml (will persist on change)"
            );
            m
        };

        let store = Self {
            path,
            inner: RwLock::new(map),
        };
        // Persist seed so Admin/UI always has a file-backed source of truth.
        if !store.path.exists() {
            store.persist()?;
        }
        Ok(store)
    }

    fn persist_locked(&self, map: &HashMap<String, UpstreamRecord>) -> Result<(), StoreError> {
        let mut list: Vec<&UpstreamRecord> = map.values().collect();
        list.sort_by(|a, b| a.id.cmp(&b.id));
        let text = serde_json::to_string_pretty(&list)
            .map_err(|e| StoreError::Other(e.to_string()))?;
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).map_err(|e| StoreError::Other(e.to_string()))?;
        }
        fs::write(&self.path, text).map_err(|e| StoreError::Other(e.to_string()))?;
        Ok(())
    }

    fn persist(&self) -> anyhow::Result<()> {
        let map = self
            .inner
            .read()
            .map_err(|_| anyhow::anyhow!("lock poisoned"))?;
        self.persist_locked(&map)
            .map_err(|e| anyhow::anyhow!(e.to_string()))
    }

    pub fn list(&self) -> Result<Vec<UpstreamRecord>, StoreError> {
        let map = self
            .inner
            .read()
            .map_err(|_| StoreError::Other("lock poisoned".into()))?;
        let mut list: Vec<UpstreamRecord> = map.values().cloned().collect();
        list.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(list)
    }

    pub fn get(&self, id: &str) -> Result<Upstream, StoreError> {
        let map = self
            .inner
            .read()
            .map_err(|_| StoreError::Other("lock poisoned".into()))?;
        map.get(id)
            .map(|r| r.to_upstream())
            .ok_or(StoreError::NotFound)
    }

    pub fn create(&self, req: UpsertUpstreamRequest) -> Result<UpstreamRecord, StoreError> {
        let name = req
            .name
            .unwrap_or_else(|| req.id.clone())
            .trim()
            .to_string();
        let rec = UpstreamRecord {
            id: req.id.trim().to_string(),
            name: if name.is_empty() {
                req.id.trim().to_string()
            } else {
                name
            },
            base_url: req.base_url.trim().trim_end_matches('/').to_string(),
            token: req.token.trim().to_string(),
        };
        rec.validate()?;

        // Reject empty after trim (validate already checks, but keep explicit).
        if rec.token.is_empty() {
            return Err(StoreError::Other("token is required".into()));
        }

        let mut map = self
            .inner
            .write()
            .map_err(|_| StoreError::Other("lock poisoned".into()))?;
        if map.contains_key(&rec.id) {
            return Err(StoreError::Duplicate(rec.id));
        }
        map.insert(rec.id.clone(), rec.clone());
        self.persist_locked(&map)?;
        Ok(rec)
    }

    pub fn update(&self, id: &str, req: UpdateUpstreamRequest) -> Result<UpstreamRecord, StoreError> {
        let mut map = self
            .inner
            .write()
            .map_err(|_| StoreError::Other("lock poisoned".into()))?;
        let rec = map.get_mut(id).ok_or(StoreError::NotFound)?;
        if let Some(name) = req.name {
            let name = name.trim().to_string();
            if name.is_empty() {
                return Err(StoreError::Other("name cannot be empty".into()));
            }
            rec.name = name;
        }
        if let Some(url) = req.base_url {
            let url = url.trim().trim_end_matches('/').to_string();
            if url.is_empty() {
                return Err(StoreError::Other("base_url cannot be empty".into()));
            }
            rec.base_url = url;
        }
        if let Some(token) = req.token {
            let token = token.trim().to_string();
            if token.is_empty() {
                return Err(StoreError::Other("token cannot be empty".into()));
            }
            rec.token = token;
        }
        let out = rec.clone();
        out.validate()?;
        self.persist_locked(&map)?;
        Ok(out)
    }

    pub fn remove(&self, id: &str) -> Result<UpstreamRecord, StoreError> {
        let mut map = self
            .inner
            .write()
            .map_err(|_| StoreError::Other("lock poisoned".into()))?;
        let rec = map.remove(id).ok_or(StoreError::NotFound)?;
        self.persist_locked(&map)?;
        Ok(rec)
    }
}
