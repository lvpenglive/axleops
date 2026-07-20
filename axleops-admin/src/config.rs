use serde::Deserialize;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct Config {
    /// Listen address, e.g. 0.0.0.0:9000
    pub bind: String,
    /// Legacy / service bootstrap token (`X-AxleOps-Token`). Still accepted for automation.
    pub token: String,
    /// Where SQLite and runtime data live
    pub data_dir: PathBuf,
    /// Created on first boot when users table is empty
    pub seed_username: String,
    pub seed_password: String,
    /// Session lifetime for username/password login
    pub session_ttl_hours: u64,
    /// Browser CORS allow-list.
    /// Empty or containing `"*"` → permissive (dev-friendly).
    /// Otherwise only listed origins (e.g. `["https://ops.example.com"]`).
    pub cors_origins: Vec<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            bind: "0.0.0.0:9000".into(),
            token: "change-me-to-a-long-secret".into(),
            data_dir: PathBuf::from("./data"),
            seed_username: "admin".into(),
            seed_password: "change-me-admin".into(),
            session_ttl_hours: 24,
            cors_origins: Vec::new(),
        }
    }
}

impl Config {
    pub fn load() -> anyhow::Result<Self> {
        let mut cfg = if let Ok(content) = fs::read_to_string("config.toml") {
            toml::from_str::<Config>(&content)?
        } else {
            Config::default()
        };

        if let Ok(v) = std::env::var("AXLEOPS_ADMIN_BIND") {
            cfg.bind = v;
        }
        if let Ok(v) = std::env::var("AXLEOPS_ADMIN_TOKEN") {
            cfg.token = v;
        }
        if let Ok(v) = std::env::var("AXLEOPS_ADMIN_DATA_DIR") {
            cfg.data_dir = PathBuf::from(v);
        }
        if let Ok(v) = std::env::var("AXLEOPS_ADMIN_SEED_USERNAME") {
            cfg.seed_username = v;
        }
        if let Ok(v) = std::env::var("AXLEOPS_ADMIN_SEED_PASSWORD") {
            cfg.seed_password = v;
        }
        if let Ok(v) = std::env::var("AXLEOPS_ADMIN_SESSION_TTL_HOURS") {
            if let Ok(n) = v.parse() {
                cfg.session_ttl_hours = n;
            }
        }
        if let Ok(v) = std::env::var("AXLEOPS_ADMIN_CORS_ORIGINS") {
            cfg.cors_origins = v
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
        }

        fs::create_dir_all(&cfg.data_dir)?;
        Ok(cfg)
    }

    pub fn cors_is_permissive(&self) -> bool {
        self.cors_origins.is_empty()
            || self
                .cors_origins
                .iter()
                .any(|o| o.trim() == "*")
    }
}
