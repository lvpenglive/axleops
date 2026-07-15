mod auth;
mod config;
mod models;
mod process;
mod routes;

use config::Config;
use process::ProcessManager;
use std::sync::Arc;
use tower_http::trace::TraceLayer;
use tracing_subscriber::EnvFilter;

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub processes: Arc<ProcessManager>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .init();

    let config = Config::load()?;
    let bind = config.bind.clone();
    tracing::info!(
        bind = %bind,
        data_dir = %config.data_dir.display(),
        "starting axleops-agent"
    );

    let state = AppState {
        processes: Arc::new(ProcessManager::new(config.data_dir.clone())),
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
