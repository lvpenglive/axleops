use crate::auth::AuthToken;
use crate::models::{ApiResponse, HealthInfo, ServiceSpec, ServiceStatus};
use crate::AppState;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::routing::{get, post, put};
use axum::{Json, Router};
use serde::Deserialize;

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

async fn list_services(
    _auth: AuthToken,
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<Vec<ServiceStatus>>>, (StatusCode, Json<ApiResponse<()>>)> {
    match state.processes.list_statuses() {
        Ok(list) => Ok(Json(ApiResponse::ok("ok", list))),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<()>::err(e.to_string())),
        )),
    }
}

async fn service_status(
    _auth: AuthToken,
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Json<ApiResponse<ServiceStatus>> {
    Json(ApiResponse::ok("ok", state.processes.status(&name)))
}

async fn get_service_spec(
    _auth: AuthToken,
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<Json<ApiResponse<ServiceSpec>>, (StatusCode, Json<ApiResponse<()>>)> {
    match state.processes.get_spec(&name) {
        Ok(spec) => Ok(Json(ApiResponse::ok("ok", spec))),
        Err(e) => Err((
            StatusCode::NOT_FOUND,
            Json(ApiResponse::<()>::err(e.to_string())),
        )),
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
    match state.processes.save(&spec) {
        Ok(status) => Ok(Json(ApiResponse::ok("saved", status))),
        Err(e) => Err((
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::<()>::err(e.to_string())),
        )),
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
    match state.processes.save(&spec) {
        Ok(status) => Ok(Json(ApiResponse::ok("saved", status))),
        Err(e) => Err((
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::<()>::err(e.to_string())),
        )),
    }
}

async fn remove_service(
    _auth: AuthToken,
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<Json<ApiResponse<ServiceStatus>>, (StatusCode, Json<ApiResponse<()>>)> {
    match state.processes.remove(&name) {
        Ok(status) => Ok(Json(ApiResponse::ok("removed", status))),
        Err(e) => Err((
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::<()>::err(e.to_string())),
        )),
    }
}

async fn start_service(
    _auth: AuthToken,
    State(state): State<AppState>,
    Json(spec): Json<ServiceSpec>,
) -> Result<Json<ApiResponse<ServiceStatus>>, (StatusCode, Json<ApiResponse<()>>)> {
    validate_spec(&spec)?;
    match state.processes.start(&spec) {
        Ok(status) => Ok(Json(ApiResponse::ok("started", status))),
        Err(e) => Err((
            StatusCode::CONFLICT,
            Json(ApiResponse::<()>::err(e.to_string())),
        )),
    }
}

async fn start_saved_service(
    _auth: AuthToken,
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<Json<ApiResponse<ServiceStatus>>, (StatusCode, Json<ApiResponse<()>>)> {
    match state.processes.start_saved(&name) {
        Ok(status) => Ok(Json(ApiResponse::ok("started", status))),
        Err(e) => Err((
            StatusCode::CONFLICT,
            Json(ApiResponse::<()>::err(e.to_string())),
        )),
    }
}

async fn stop_service(
    _auth: AuthToken,
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<Json<ApiResponse<ServiceStatus>>, (StatusCode, Json<ApiResponse<()>>)> {
    match state.processes.stop(&name) {
        Ok(status) => Ok(Json(ApiResponse::ok("stopped", status))),
        Err(e) => Err((
            StatusCode::CONFLICT,
            Json(ApiResponse::<()>::err(e.to_string())),
        )),
    }
}

async fn restart_service(
    _auth: AuthToken,
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<Json<ApiResponse<ServiceStatus>>, (StatusCode, Json<ApiResponse<()>>)> {
    match state.processes.restart(&name) {
        Ok(status) => Ok(Json(ApiResponse::ok("restarted", status))),
        Err(e) => Err((
            StatusCode::CONFLICT,
            Json(ApiResponse::<()>::err(e.to_string())),
        )),
    }
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
    match state.processes.tail_log(&name, q.bytes) {
        Ok(content) => Ok(Json(ApiResponse::ok("ok", content))),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<()>::err(e.to_string())),
        )),
    }
}
