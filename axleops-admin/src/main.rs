mod agents;
mod auth;
mod config;
mod models;
mod proxy;
mod routes;

use agents::AgentRegistry;
use config::Config;
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing_subscriber::EnvFilter;

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub agents: Arc<AgentRegistry>,
    pub http: reqwest::Client,
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
        "starting axleops-admin"
    );

    let agents = AgentRegistry::load(&config.data_dir)?;
    let http = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()?;

    let state = AppState {
        agents: Arc::new(agents),
        config: Arc::new(config),
        http,
    };

    let app = routes::router()
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
        .with_state(state);

    if !std::path::Path::new("static/index.html").exists() {
        tracing::warn!("static/index.html not found; open UI from the axleops-admin working directory");
    }

    let listener = tokio::net::TcpListener::bind(&bind).await?;
    tracing::info!("listening on http://{bind}");
    axum::serve(listener, app).await?;
    Ok(())
}
