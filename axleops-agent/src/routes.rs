use crate::auth::AuthToken;
use crate::models::{ApiResponse, HealthInfo, ServiceSpec, ServiceStatus};
use crate::process::{ProcessError, ProcessManager};
use crate::AppState;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::routing::{get, post, put};
use axum::{Json, Router};
use serde::Deserialize;
use std::sync::Arc;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/health", get(health))
        .route("/api/v1/services", get(list_services).post(save_service))
        .route("/api/v1/services/{name}/status", get(service_status))
        .route("/api/v1/services/{name}/spec", get(get_service_spec))
        .route(
            "/api/v1/services/{name}",
            put(save_service_named).delete(remove_service),
        )
        .route("/api/v1/services/start", post(start_service))
        .route("/api/v1/services/{name}/start", post(start_saved_service))
        .route("/api/v1/services/{name}/stop", post(stop_service))
        .route("/api/v1/services/{name}/restart", post(restart_service))
        .route("/api/v1/services/{name}/logs", get(service_logs))
}

async fn health() -> Json<ApiResponse<HealthInfo>> {
    Json(ApiResponse::ok(
        "healthy",
        HealthInfo {
            service: "axleops-agent",
            version: env!("CARGO_PKG_VERSION"),
        },
    ))
}

async fn run_blocking<T, F>(
    processes: Arc<ProcessManager>,
    f: F,
) -> Result<T, (StatusCode, Json<ApiResponse<()>>)>
where
    T: Send + 'static,
    F: FnOnce(Arc<ProcessManager>) -> Result<T, ProcessError> + Send + 'static,
{
    tokio::task::spawn_blocking(move || f(processes))
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<()>::err(format!("blocking task failed: {e}"))),
            )
        })?
        .map_err(|e| map_process_err(e))
}

fn map_process_err(e: ProcessError) -> (StatusCode, Json<ApiResponse<()>>) {
    let status = match &e {
        ProcessError::AlreadyRunning(_) | ProcessError::NotRunning => StatusCode::CONFLICT,
        ProcessError::Other(_) | ProcessError::InvalidPidFile | ProcessError::Io(_) => {
            // Keep prior behavior: start/stop conflicts vs save bad requests handled per-route.
            StatusCode::CONFLICT
        }
    };
    (status, Json(ApiResponse::<()>::err(e.to_string())))
}

async fn list_services(
    _auth: AuthToken,
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<Vec<ServiceStatus>>>, (StatusCode, Json<ApiResponse<()>>)> {
    let list = run_blocking(state.processes.clone(), |p| p.list_statuses()).await?;
    Ok(Json(ApiResponse::ok("ok", list)))
}

async fn service_status(
    _auth: AuthToken,
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<Json<ApiResponse<ServiceStatus>>, (StatusCode, Json<ApiResponse<()>>)> {
    let status = run_blocking(state.processes.clone(), move |p| Ok(p.status(&name))).await?;
    Ok(Json(ApiResponse::ok("ok", status)))
}

async fn get_service_spec(
    _auth: AuthToken,
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<Json<ApiResponse<ServiceSpec>>, (StatusCode, Json<ApiResponse<()>>)> {
    match run_blocking(state.processes.clone(), move |p| p.get_spec(&name)).await {
        Ok(spec) => Ok(Json(ApiResponse::ok("ok", spec))),
        Err((_, json)) => Err((StatusCode::NOT_FOUND, json)),
    }
}

fn validate_spec(spec: &ServiceSpec) -> Result<(), (StatusCode, Json<ApiResponse<()>>)> {
    if spec.name.trim().is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::<()>::err("name is required")),
        ));
    }
    if let Err(e) = spec.resolved_kind() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::<()>::err(e)),
        ));
    }
    Ok(())
}

async fn save_service(
    _auth: AuthToken,
    State(state): State<AppState>,
    Json(spec): Json<ServiceSpec>,
) -> Result<Json<ApiResponse<ServiceStatus>>, (StatusCode, Json<ApiResponse<()>>)> {
    validate_spec(&spec)?;
    match run_blocking(state.processes.clone(), move |p| p.save(&spec)).await {
        Ok(status) => Ok(Json(ApiResponse::ok("saved", status))),
        Err((_, json)) => Err((StatusCode::BAD_REQUEST, json)),
    }
}

async fn save_service_named(
    _auth: AuthToken,
    State(state): State<AppState>,
    Path(name): Path<String>,
    Json(mut spec): Json<ServiceSpec>,
) -> Result<Json<ApiResponse<ServiceStatus>>, (StatusCode, Json<ApiResponse<()>>)> {
    spec.name = name;
    validate_spec(&spec)?;
    match run_blocking(state.processes.clone(), move |p| p.save(&spec)).await {
        Ok(status) => Ok(Json(ApiResponse::ok("saved", status))),
        Err((_, json)) => Err((StatusCode::BAD_REQUEST, json)),
    }
}

async fn remove_service(
    _auth: AuthToken,
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<Json<ApiResponse<ServiceStatus>>, (StatusCode, Json<ApiResponse<()>>)> {
    match run_blocking(state.processes.clone(), move |p| p.remove(&name)).await {
        Ok(status) => Ok(Json(ApiResponse::ok("removed", status))),
        Err((_, json)) => Err((StatusCode::BAD_REQUEST, json)),
    }
}

async fn start_service(
    _auth: AuthToken,
    State(state): State<AppState>,
    Json(spec): Json<ServiceSpec>,
) -> Result<Json<ApiResponse<ServiceStatus>>, (StatusCode, Json<ApiResponse<()>>)> {
    validate_spec(&spec)?;
    let status = run_blocking(state.processes.clone(), move |p| p.start(&spec)).await?;
    Ok(Json(ApiResponse::ok("started", status)))
}

async fn start_saved_service(
    _auth: AuthToken,
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<Json<ApiResponse<ServiceStatus>>, (StatusCode, Json<ApiResponse<()>>)> {
    let status = run_blocking(state.processes.clone(), move |p| p.start_saved(&name)).await?;
    Ok(Json(ApiResponse::ok("started", status)))
}

async fn stop_service(
    _auth: AuthToken,
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<Json<ApiResponse<ServiceStatus>>, (StatusCode, Json<ApiResponse<()>>)> {
    let status = run_blocking(state.processes.clone(), move |p| p.stop(&name)).await?;
    Ok(Json(ApiResponse::ok("stopped", status)))
}

async fn restart_service(
    _auth: AuthToken,
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<Json<ApiResponse<ServiceStatus>>, (StatusCode, Json<ApiResponse<()>>)> {
    let status = run_blocking(state.processes.clone(), move |p| p.restart(&name)).await?;
    Ok(Json(ApiResponse::ok("restarted", status)))
}

#[derive(Debug, Deserialize)]
struct LogQuery {
    /// Max bytes to return from end of log file (default 32KB)
    #[serde(default = "default_log_bytes")]
    bytes: u64,
}

fn default_log_bytes() -> u64 {
    32 * 1024
}

async fn service_logs(
    _auth: AuthToken,
    State(state): State<AppState>,
    Path(name): Path<String>,
    Query(q): Query<LogQuery>,
) -> Result<Json<ApiResponse<String>>, (StatusCode, Json<ApiResponse<()>>)> {
    let content = run_blocking(state.processes.clone(), move |p| p.tail_log(&name, q.bytes)).await
        .map_err(|(_, json)| (StatusCode::INTERNAL_SERVER_ERROR, json))?;
    Ok(Json(ApiResponse::ok("ok", content)))
}
