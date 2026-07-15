use serde::Deserialize;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct Config {
    /// Listen address, e.g. 0.0.0.0:9100
    pub bind: String,
    /// Shared secret; clients must send header X-AxleOps-Token
    pub token: String,
    /// Where PID files and service definitions live
    pub data_dir: PathBuf,
    /// Periodically check desired services and restart if dead/unhealthy
    pub watchdog_enabled: bool,
    /// Watchdog interval in seconds
    pub watchdog_interval_secs: u64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            bind: "0.0.0.0:9100".into(),
            token: "change-me-to-a-long-secret".into(),
            data_dir: PathBuf::from("./data"),
            watchdog_enabled: true,
            watchdog_interval_secs: 15,
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

        if let Ok(v) = std::env::var("AXLEOPS_BIND") {
            cfg.bind = v;
        }
        if let Ok(v) = std::env::var("AXLEOPS_TOKEN") {
            cfg.token = v;
        }
        if let Ok(v) = std::env::var("AXLEOPS_DATA_DIR") {
            cfg.data_dir = PathBuf::from(v);
        }
        if let Ok(v) = std::env::var("AXLEOPS_WATCHDOG_ENABLED") {
            cfg.watchdog_enabled = matches!(
                v.to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            );
        }
        if let Ok(v) = std::env::var("AXLEOPS_WATCHDOG_INTERVAL_SECS") {
            if let Ok(n) = v.parse() {
                cfg.watchdog_interval_secs = n;
            }
        }

        fs::create_dir_all(&cfg.data_dir)?;
        fs::create_dir_all(cfg.data_dir.join("pids"))?;
        fs::create_dir_all(cfg.data_dir.join("logs"))?;
        fs::create_dir_all(cfg.data_dir.join("services"))?;

        Ok(cfg)
    }
}
