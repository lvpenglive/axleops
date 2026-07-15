use crate::auth::AuthToken;
use crate::models::{
    AgentHealthView, AgentInfo, ApiResponse, HealthInfo, RegisterAgentRequest,
    StartServiceRequest, UpdateAgentRequest,
};
use crate::{agents::RegistryError, proxy::ProxyError, AppState};
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::Value;
use tower_http::services::{ServeDir, ServeFile};

pub fn router() -> Router<AppState> {
    let api = Router::new()
        .route("/health", get(health))
        .route("/api/v1/agents", get(list_agents).post(register_agent))
        .route(
            "/api/v1/agents/{id}",
            get(get_agent).put(update_agent).delete(delete_agent),
        )
        .route("/api/v1/agents/{id}/ping", get(ping_agent))
        .route(
            "/api/v1/agents/{id}/services",
            get(list_services).post(save_service),
        )
        .route("/api/v1/agents/{id}/services/start", post(start_service))
        .route(
            "/api/v1/agents/{id}/services/{name}/spec",
            get(get_service_spec),
        )
        .route(
            "/api/v1/agents/{id}/services/{name}",
            axum::routing::put(save_service_named).delete(remove_service),
        )
        .route(
            "/api/v1/agents/{id}/services/{name}/start",
            post(start_saved_service),
        )
        .route(
            "/api/v1/agents/{id}/services/{name}/restart",
            post(restart_service),
        )
        .route(
            "/api/v1/agents/{id}/services/{name}/status",
            get(service_status),
        )
        .route(
            "/api/v1/agents/{id}/services/{name}/stop",
            post(stop_service),
        )
        .route(
            "/api/v1/agents/{id}/services/{name}/logs",
            get(service_logs),
        );

    let static_files = ServeDir::new("static")
        .not_found_service(ServeFile::new("static/index.html"));

    api.fallback_service(static_files)
}

async fn health() -> Json<ApiResponse<HealthInfo>> {
    Json(ApiResponse::ok(
        "healthy",
        HealthInfo {
            service: "axleops-admin",
            version: env!("CARGO_PKG_VERSION"),
        },
    ))
}

async fn list_agents(
    _auth: AuthToken,
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<Vec<AgentInfo>>>, (StatusCode, Json<ApiResponse<()>>)> {
    match state.agents.list() {
        Ok(list) => Ok(Json(ApiResponse::ok("ok", mask_tokens(list)))),
        Err(e) => Err(registry_err(e)),
    }
}

async fn get_agent(
    _auth: AuthToken,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<AgentInfo>>, (StatusCode, Json<ApiResponse<()>>)> {
    match state.agents.get(&id) {
        Ok(agent) => Ok(Json(ApiResponse::ok("ok", mask_token(agent)))),
        Err(e) => Err(registry_err(e)),
    }
}

async fn register_agent(
    _auth: AuthToken,
    State(state): State<AppState>,
    Json(req): Json<RegisterAgentRequest>,
) -> Result<Json<ApiResponse<AgentInfo>>, (StatusCode, Json<ApiResponse<()>>)> {
    match state.agents.register(req) {
        Ok(agent) => Ok(Json(ApiResponse::ok("registered", mask_token(agent)))),
        Err(e) => Err(registry_err(e)),
    }
}

async fn update_agent(
    _auth: AuthToken,
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<UpdateAgentRequest>,
) -> Result<Json<ApiResponse<AgentInfo>>, (StatusCode, Json<ApiResponse<()>>)> {
    match state.agents.update(&id, req) {
        Ok(agent) => Ok(Json(ApiResponse::ok("updated", mask_token(agent)))),
        Err(e) => Err(registry_err(e)),
    }
}

async fn delete_agent(
    _auth: AuthToken,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<AgentInfo>>, (StatusCode, Json<ApiResponse<()>>)> {
    match state.agents.remove(&id) {
        Ok(agent) => Ok(Json(ApiResponse::ok("deleted", mask_token(agent)))),
        Err(e) => Err(registry_err(e)),
    }
}

async fn ping_agent(
    _auth: AuthToken,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<AgentHealthView>>, (StatusCode, Json<ApiResponse<()>>)> {
    let agent = state.agents.get(&id).map_err(registry_err)?;
    match crate::proxy::agent_health(&state.http, &agent).await {
        Ok(detail) => Ok(Json(ApiResponse::ok(
            "reachable",
            AgentHealthView {
                agent_id: agent.id,
                agent_name: agent.name,
                reachable: true,
                detail,
            },
        ))),
        Err(e) => Ok(Json(ApiResponse::ok(
            "unreachable",
            AgentHealthView {
                agent_id: agent.id,
                agent_name: agent.name,
                reachable: false,
                detail: Value::String(e.to_string()),
            },
        ))),
    }
}

async fn list_services(
    _auth: AuthToken,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<Value>>, (StatusCode, Json<ApiResponse<()>>)> {
    let agent = state.agents.get(&id).map_err(registry_err)?;
    match crate::proxy::agent_get(&state.http, &agent, "/api/v1/services").await {
        Ok(v) => Ok(Json(ApiResponse::ok("ok", v))),
        Err(e) => Err(proxy_err(e)),
    }
}

async fn start_service(
    _auth: AuthToken,
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<StartServiceRequest>,
) -> Result<Json<ApiResponse<Value>>, (StatusCode, Json<ApiResponse<()>>)> {
    let agent = state.agents.get(&id).map_err(registry_err)?;
    match crate::proxy::agent_post_json(&state.http, &agent, "/api/v1/services/start", &body)
        .await
    {
        Ok(v) => Ok(Json(ApiResponse::ok("ok", v))),
        Err(e) => Err(proxy_err(e)),
    }
}

async fn save_service(
    _auth: AuthToken,
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<StartServiceRequest>,
) -> Result<Json<ApiResponse<Value>>, (StatusCode, Json<ApiResponse<()>>)> {
    let agent = state.agents.get(&id).map_err(registry_err)?;
    match crate::proxy::agent_post_json(&state.http, &agent, "/api/v1/services", &body).await {
        Ok(v) => Ok(Json(ApiResponse::ok("ok", v))),
        Err(e) => Err(proxy_err(e)),
    }
}

async fn save_service_named(
    _auth: AuthToken,
    State(state): State<AppState>,
    Path((id, name)): Path<(String, String)>,
    Json(mut body): Json<StartServiceRequest>,
) -> Result<Json<ApiResponse<Value>>, (StatusCode, Json<ApiResponse<()>>)> {
    body.name = name.clone();
    let agent = state.agents.get(&id).map_err(registry_err)?;
    let path = format!("/api/v1/services/{name}");
    match crate::proxy::agent_put_json(&state.http, &agent, &path, &body).await {
        Ok(v) => Ok(Json(ApiResponse::ok("ok", v))),
        Err(e) => Err(proxy_err(e)),
    }
}

async fn get_service_spec(
    _auth: AuthToken,
    State(state): State<AppState>,
    Path((id, name)): Path<(String, String)>,
) -> Result<Json<ApiResponse<Value>>, (StatusCode, Json<ApiResponse<()>>)> {
    let agent = state.agents.get(&id).map_err(registry_err)?;
    let path = format!("/api/v1/services/{name}/spec");
    match crate::proxy::agent_get(&state.http, &agent, &path).await {
        Ok(v) => Ok(Json(ApiResponse::ok("ok", v))),
        Err(e) => Err(proxy_err(e)),
    }
}

async fn start_saved_service(
    _auth: AuthToken,
    State(state): State<AppState>,
    Path((id, name)): Path<(String, String)>,
) -> Result<Json<ApiResponse<Value>>, (StatusCode, Json<ApiResponse<()>>)> {
    let agent = state.agents.get(&id).map_err(registry_err)?;
    let path = format!("/api/v1/services/{name}/start");
    match crate::proxy::agent_post_empty(&state.http, &agent, &path).await {
        Ok(v) => Ok(Json(ApiResponse::ok("ok", v))),
        Err(e) => Err(proxy_err(e)),
    }
}

async fn restart_service(
    _auth: AuthToken,
    State(state): State<AppState>,
    Path((id, name)): Path<(String, String)>,
) -> Result<Json<ApiResponse<Value>>, (StatusCode, Json<ApiResponse<()>>)> {
    let agent = state.agents.get(&id).map_err(registry_err)?;
    let path = format!("/api/v1/services/{name}/restart");
    match crate::proxy::agent_post_empty(&state.http, &agent, &path).await {
        Ok(v) => Ok(Json(ApiResponse::ok("ok", v))),
        Err(e) => Err(proxy_err(e)),
    }
}

async fn remove_service(
    _auth: AuthToken,
    State(state): State<AppState>,
    Path((id, name)): Path<(String, String)>,
) -> Result<Json<ApiResponse<Value>>, (StatusCode, Json<ApiResponse<()>>)> {
    let agent = state.agents.get(&id).map_err(registry_err)?;
    let path = format!("/api/v1/services/{name}");
    match crate::proxy::agent_delete(&state.http, &agent, &path).await {
        Ok(v) => Ok(Json(ApiResponse::ok("ok", v))),
        Err(e) => Err(proxy_err(e)),
    }
}

async fn service_status(
    _auth: AuthToken,
    State(state): State<AppState>,
    Path((id, name)): Path<(String, String)>,
) -> Result<Json<ApiResponse<Value>>, (StatusCode, Json<ApiResponse<()>>)> {
    let agent = state.agents.get(&id).map_err(registry_err)?;
    let path = format!("/api/v1/services/{name}/status");
    match crate::proxy::agent_get(&state.http, &agent, &path).await {
        Ok(v) => Ok(Json(ApiResponse::ok("ok", v))),
        Err(e) => Err(proxy_err(e)),
    }
}

async fn stop_service(
    _auth: AuthToken,
    State(state): State<AppState>,
    Path((id, name)): Path<(String, String)>,
) -> Result<Json<ApiResponse<Value>>, (StatusCode, Json<ApiResponse<()>>)> {
    let agent = state.agents.get(&id).map_err(registry_err)?;
    let path = format!("/api/v1/services/{name}/stop");
    match crate::proxy::agent_post_empty(&state.http, &agent, &path).await {
        Ok(v) => Ok(Json(ApiResponse::ok("ok", v))),
        Err(e) => Err(proxy_err(e)),
    }
}

#[derive(Debug, Deserialize)]
struct LogQuery {
    #[serde(default = "default_log_bytes")]
    bytes: u64,
}

fn default_log_bytes() -> u64 {
    32 * 1024
}

async fn service_logs(
    _auth: AuthToken,
    State(state): State<AppState>,
    Path((id, name)): Path<(String, String)>,
    Query(q): Query<LogQuery>,
) -> Result<Json<ApiResponse<Value>>, (StatusCode, Json<ApiResponse<()>>)> {
    let agent = state.agents.get(&id).map_err(registry_err)?;
    let path = format!("/api/v1/services/{name}/logs?bytes={}", q.bytes);
    match crate::proxy::agent_get(&state.http, &agent, &path).await {
        Ok(v) => Ok(Json(ApiResponse::ok("ok", v))),
        Err(e) => Err(proxy_err(e)),
    }
}

fn mask_token(mut agent: AgentInfo) -> AgentInfo {
    if agent.token.len() > 4 {
        agent.token = format!("{}****", &agent.token[..4]);
    } else {
        agent.token = "****".into();
    }
    agent
}

fn mask_tokens(list: Vec<AgentInfo>) -> Vec<AgentInfo> {
    list.into_iter().map(mask_token).collect()
}

fn registry_err(e: RegistryError) -> (StatusCode, Json<ApiResponse<()>>) {
    let status = match &e {
        RegistryError::NotFound => StatusCode::NOT_FOUND,
        RegistryError::DuplicateName(_) => StatusCode::CONFLICT,
        RegistryError::Other(_) => StatusCode::BAD_REQUEST,
        RegistryError::Io(_) | RegistryError::Db(_) => StatusCode::INTERNAL_SERVER_ERROR,
    };
    (status, Json(ApiResponse::<()>::err(e.to_string())))
}

fn proxy_err(e: ProxyError) -> (StatusCode, Json<ApiResponse<()>>) {
    let status = match &e {
        ProxyError::AgentStatus { status, .. } => {
            StatusCode::from_u16(status.as_u16()).unwrap_or(StatusCode::BAD_GATEWAY)
        }
        ProxyError::Http(_) => StatusCode::BAD_GATEWAY,
        ProxyError::Other(_) => StatusCode::INTERNAL_SERVER_ERROR,
    };
    (status, Json(ApiResponse::<()>::err(e.to_string())))
}
