use crate::artifacts::{ArtifactInfo, ArtifactList, PublishResult};
use crate::auth::AuthToken;
use crate::models::{ApiResponse, HealthInfo, ServiceSpec, ServiceStatus};
use crate::process::{ProcessError, ProcessManager};
use crate::AppState;
use axum::extract::{DefaultBodyLimit, Multipart, Path, Query, State};
use axum::http::StatusCode;
use axum::routing::{get, post, put};
use axum::{Json, Router};
use serde::Deserialize;
use std::sync::Arc;

/// 512 MiB upload cap for JAR / script artifacts.
const ARTIFACT_BODY_LIMIT: usize = 512 * 1024 * 1024;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/health", get(health))
        .route("/api/v1/auth/rotate-token", post(rotate_token))
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
        .route(
            "/api/v1/services/{name}/artifacts",
            get(list_artifacts).post(upload_artifact),
        )
        .route(
            "/api/v1/services/{name}/artifacts/{version}/activate",
            post(activate_artifact),
        )
        .route(
            "/api/v1/services/{name}/artifacts/{version}",
            axum::routing::delete(delete_artifact),
        )
        .route("/api/v1/services/{name}/publish", post(publish_service))
        .route("/api/v1/services/{name}/rollback", post(rollback_service))
        .layer(DefaultBodyLimit::max(ARTIFACT_BODY_LIMIT))
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

async fn rotate_token(
    _auth: AuthToken,
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<serde_json::Value>>, (StatusCode, Json<ApiResponse<()>>)> {
    let new_token = format!("axle_{}", uuid::Uuid::new_v4().simple());
    if let Err(e) = state.config.persist_token(&new_token) {
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<()>::err(format!("persist token failed: {e}"))),
        ));
    }
    {
        let mut guard = state.token.write().map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<()>::err("token lock poisoned")),
            )
        })?;
        *guard = new_token.clone();
    }
    tracing::warn!("agent token rotated; clients must use the new token");
    Ok(Json(ApiResponse::ok(
        "rotated",
        serde_json::json!({ "token": new_token }),
    )))
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

async fn list_artifacts(
    _auth: AuthToken,
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<Json<ApiResponse<ArtifactList>>, (StatusCode, Json<ApiResponse<()>>)> {
    let store = state.artifacts.clone();
    let list = tokio::task::spawn_blocking(move || store.list(&name))
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<()>::err(format!("blocking task failed: {e}"))),
            )
        })?
        .map_err(map_process_err)?;
    Ok(Json(ApiResponse::ok("ok", list)))
}

async fn upload_artifact(
    _auth: AuthToken,
    State(state): State<AppState>,
    Path(name): Path<String>,
    mut multipart: Multipart,
) -> Result<Json<ApiResponse<ArtifactInfo>>, (StatusCode, Json<ApiResponse<()>>)> {
    let mut file_name: Option<String> = None;
    let mut file_bytes: Option<Vec<u8>> = None;
    let mut version: Option<String> = None;
    let mut note: Option<String> = None;

    while let Some(field) = multipart.next_field().await.map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::<()>::err(format!("multipart error: {e}"))),
        )
    })? {
        let field_name = field.name().unwrap_or("").to_string();
        match field_name.as_str() {
            "file" | "artifact" => {
                file_name = field
                    .file_name()
                    .map(|s| s.to_string())
                    .filter(|s| !s.is_empty());
                let data = field.bytes().await.map_err(|e| {
                    (
                        StatusCode::BAD_REQUEST,
                        Json(ApiResponse::<()>::err(format!("read file failed: {e}"))),
                    )
                })?;
                file_bytes = Some(data.to_vec());
            }
            "version" | "id" => {
                let text = field.text().await.unwrap_or_default();
                if !text.trim().is_empty() {
                    version = Some(text.trim().to_string());
                }
            }
            "note" => {
                let text = field.text().await.unwrap_or_default();
                if !text.trim().is_empty() {
                    note = Some(text.trim().to_string());
                }
            }
            _ => {}
        }
    }

    let bytes = file_bytes.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::<()>::err("multipart field `file` is required")),
        )
    })?;
    let filename = file_name.unwrap_or_else(|| "artifact.bin".into());
    let store = state.artifacts.clone();
    let info = tokio::task::spawn_blocking(move || {
        store.store(
            &name,
            &filename,
            &bytes,
            version.as_deref(),
            note.as_deref(),
        )
    })
    .await
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<()>::err(format!("blocking task failed: {e}"))),
        )
    })?
    .map_err(map_process_err)?;

    Ok(Json(ApiResponse::ok("uploaded", info)))
}

#[derive(Debug, Deserialize)]
struct PublishBody {
    version: String,
    #[serde(default = "default_true")]
    start: bool,
}

#[derive(Debug, Deserialize)]
struct RollbackBody {
    #[serde(default = "default_true")]
    start: bool,
}

fn default_true() -> bool {
    true
}

async fn activate_artifact(
    _auth: AuthToken,
    State(state): State<AppState>,
    Path((name, version)): Path<(String, String)>,
) -> Result<Json<ApiResponse<ArtifactInfo>>, (StatusCode, Json<ApiResponse<()>>)> {
    let store = state.artifacts.clone();
    let processes = state.processes.clone();
    let info = tokio::task::spawn_blocking(move || store.activate(&processes, &name, &version))
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<()>::err(format!("blocking task failed: {e}"))),
            )
        })?
        .map_err(map_process_err)?;
    Ok(Json(ApiResponse::ok("activated", info)))
}

async fn delete_artifact(
    _auth: AuthToken,
    State(state): State<AppState>,
    Path((name, version)): Path<(String, String)>,
) -> Result<Json<ApiResponse<()>>, (StatusCode, Json<ApiResponse<()>>)> {
    let store = state.artifacts.clone();
    tokio::task::spawn_blocking(move || store.delete(&name, &version))
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<()>::err(format!("blocking task failed: {e}"))),
            )
        })?
        .map_err(map_process_err)?;
    Ok(Json(ApiResponse {
        ok: true,
        message: "deleted".into(),
        data: Some(()),
    }))
}

async fn publish_service(
    _auth: AuthToken,
    State(state): State<AppState>,
    Path(name): Path<String>,
    Json(body): Json<PublishBody>,
) -> Result<Json<ApiResponse<PublishResult>>, (StatusCode, Json<ApiResponse<()>>)> {
    if body.version.trim().is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::<()>::err("version is required")),
        ));
    }
    let store = state.artifacts.clone();
    let processes = state.processes.clone();
    let version = body.version.trim().to_string();
    let start = body.start;
    let result = tokio::task::spawn_blocking(move || {
        store.publish(&processes, &name, &version, start)
    })
    .await
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<()>::err(format!("blocking task failed: {e}"))),
        )
    })?
    .map_err(map_process_err)?;
    Ok(Json(ApiResponse::ok("published", result)))
}

async fn rollback_service(
    _auth: AuthToken,
    State(state): State<AppState>,
    Path(name): Path<String>,
    body: Option<Json<RollbackBody>>,
) -> Result<Json<ApiResponse<PublishResult>>, (StatusCode, Json<ApiResponse<()>>)> {
    let start = body.map(|b| b.0.start).unwrap_or(true);
    let store = state.artifacts.clone();
    let processes = state.processes.clone();
    let result =
        tokio::task::spawn_blocking(move || store.rollback(&processes, &name, start))
            .await
            .map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiResponse::<()>::err(format!("blocking task failed: {e}"))),
                )
            })?
            .map_err(map_process_err)?;
    Ok(Json(ApiResponse::ok("rolled_back", result)))
}
