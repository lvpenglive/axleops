mod auth;
mod config;
mod models;
mod process;
mod routes;

use config::Config;
use process::ProcessManager;
use std::sync::{Arc, RwLock};
use std::time::Duration;
use tower_http::trace::TraceLayer;
use tracing_subscriber::EnvFilter;

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    /// Live auth token (may be hot-rotated; also persisted under data_dir/auth.token).
    pub token: Arc<RwLock<String>>,
    pub processes: Arc<ProcessManager>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .init();

    let config = Config::load()?;
    let bind = config.bind.clone();
    let token = Arc::new(RwLock::new(config.token.clone()));
    tracing::info!(
        bind = %bind,
        data_dir = %config.data_dir.display(),
        watchdog = config.watchdog_enabled,
        "starting axleops-agent"
    );

    let processes = Arc::new(ProcessManager::new(&config));
    processes.recover_desired();

    if config.watchdog_enabled {
        let interval = Duration::from_secs(config.watchdog_interval_secs.max(5));
        let wd = processes.clone();
        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(interval);
            ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
            loop {
                ticker.tick().await;
                let mgr = wd.clone();
                // Offload blocking process status / spawn to blocking pool.
                let _ = tokio::task::spawn_blocking(move || mgr.watchdog_tick()).await;
            }
        });
        tracing::info!(
            interval_secs = interval.as_secs(),
            "watchdog enabled"
        );
    }

    let state = AppState {
        processes,
        token,
        config: Arc::new(config),
    };

    let app = routes::router()
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(&bind).await?;
    tracing::info!("listening on http://{bind}");
    axum::serve(listener, app).await?;
    Ok(())
}
