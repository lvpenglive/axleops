use serde::Deserialize;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Deserialize)]
pub struct UpstreamAgent {
    /// Path key used by Admin: /a/{id}/...
    pub id: String,
    #[serde(default)]
    pub name: Option<String>,
    /// Agent base URL, e.g. http://10.0.1.11:9100
    pub base_url: String,
    /// Token for Proxy → Agent
    pub token: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct Config {
    pub bind: String,
    /// Token for Admin → Proxy (`X-AxleOps-Token`)
    pub token: String,
    pub timeout_secs: u64,
    /// Runtime data (upstreams.json). Seeded from [[agents]] when empty.
    pub data_dir: PathBuf,
    /// Optional seed list (used only when data_dir/upstreams.json does not exist yet)
    pub agents: Vec<UpstreamAgent>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            bind: "0.0.0.0:9200".into(),
            token: "change-me-proxy-secret".into(),
            timeout_secs: 120,
            data_dir: PathBuf::from("./data"),
            agents: Vec::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Upstream {
    pub id: String,
    pub name: String,
    pub base_url: String,
    pub token: String,
}

impl Config {
    pub fn load() -> anyhow::Result<Self> {
        let mut cfg = if let Ok(content) = fs::read_to_string("config.toml") {
            toml::from_str::<Config>(&content)?
        } else {
            tracing::warn!("config.toml not found; using defaults (no upstreams)");
            Config::default()
        };

        if let Ok(v) = std::env::var("AXLEOPS_PROXY_BIND") {
            cfg.bind = v;
        }
        if let Ok(v) = std::env::var("AXLEOPS_PROXY_TOKEN") {
            cfg.token = v;
        }
        cfg.token = cfg.token.trim().to_string();
        if let Ok(v) = std::env::var("AXLEOPS_PROXY_TIMEOUT_SECS") {
            if let Ok(n) = v.parse() {
                cfg.timeout_secs = n;
            }
        }
        if let Ok(v) = std::env::var("AXLEOPS_PROXY_DATA_DIR") {
            cfg.data_dir = PathBuf::from(v);
        }

        for a in &cfg.agents {
            if a.id.trim().is_empty() {
                anyhow::bail!("agent id cannot be empty");
            }
            if a.base_url.trim().is_empty() {
                anyhow::bail!("agent `{}` base_url cannot be empty", a.id);
            }
            if a.token.trim().is_empty() {
                anyhow::bail!("agent `{}` token cannot be empty", a.id);
            }
        }

        let mut seen = std::collections::HashSet::new();
        for a in &cfg.agents {
            if !seen.insert(a.id.as_str()) {
                anyhow::bail!("duplicate agent id `{}`", a.id);
            }
        }

        fs::create_dir_all(&cfg.data_dir)?;
        Ok(cfg)
    }
}
