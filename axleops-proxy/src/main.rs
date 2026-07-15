mod auth;
mod config;
mod forward;
mod routes;
mod store;

use config::Config;
use store::UpstreamStore;
use std::sync::Arc;
use tower_http::trace::TraceLayer;
use tracing_subscriber::EnvFilter;

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub store: Arc<UpstreamStore>,
    pub http: reqwest::Client,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .init();

    let config = Config::load()?;
    let bind = config.bind.clone();
    let store = UpstreamStore::load(&config.data_dir, &config)?;
    let list = store.list().unwrap_or_default();
    tracing::info!(
        bind = %bind,
        data_dir = %config.data_dir.display(),
        agents = list.len(),
        "starting axleops-proxy"
    );
    for u in &list {
        tracing::info!(id = %u.id, name = %u.name, base_url = %u.base_url, "upstream ready");
    }

    let http = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(config.timeout_secs))
        .build()?;

    let state = AppState {
        store: Arc::new(store),
        config: Arc::new(config),
        http,
    };

    let app = routes::router()
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(&bind).await?;
    tracing::info!("listening on http://{bind}");
    axum::serve(listener, app).await?;
    Ok(())
}
