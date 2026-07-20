use crate::auth::AuthUser;
use crate::models::{
    AgentHealthView, AgentInfo, ApiResponse, CreateUpstreamRequest, HealthInfo,
    ImportUpstreamRequest, OverviewResponse, OverviewServiceRow, ProxyHealthView, ProxyInfo,
    RegisterAgentRequest, RegisterProxyRequest, RotateTokenRequest, RotateTokenResponse,
    StartServiceRequest, UpdateAgentRequest, UpdateProxyRequest, UpdateUpstreamRequest,
    UpstreamView,
};
use crate::users::{
    AuthContext, ChangePasswordRequest, CreateUserRequest, LoginRequest, LoginResponse,
    SetDisabledRequest, UserError, UserInfo,
};
use crate::{agents::RegistryError, proxies::ProxyRegistryError, proxy::ProxyError, AppState};
use axum::body::Bytes;
use axum::extract::{DefaultBodyLimit, Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tower_http::services::{ServeDir, ServeFile};

const ARTIFACT_BODY_LIMIT: usize = 512 * 1024 * 1024;

pub fn router() -> Router<AppState> {
    let api = Router::new()
        .route("/health", get(health))
        .route("/api/v1/auth/login", post(login))
        .route("/api/v1/auth/logout", post(logout))
        .route("/api/v1/auth/me", get(auth_me))
        .route("/api/v1/auth/change-password", post(change_password))
        .route("/api/v1/users", get(list_users).post(create_user))
        .route(
            "/api/v1/users/{id}/disabled",
            axum::routing::put(set_user_disabled),
        )
        .route("/api/v1/audit-logs", get(list_audit_logs))
        .route("/api/v1/overview/services", get(overview_services))
        .route("/api/v1/agents", get(list_agents).post(register_agent))
        .route(
            "/api/v1/agents/{id}",
            get(get_agent).put(update_agent).delete(delete_agent),
        )
        .route("/api/v1/agents/{id}/ping", get(ping_agent))
        .route(
            "/api/v1/agents/{id}/rotate-token",
            post(rotate_agent_token),
        )
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
        )
        .route(
            "/api/v1/agents/{id}/services/{name}/artifacts",
            get(list_artifacts).post(upload_artifact),
        )
        .route(
            "/api/v1/agents/{id}/services/{name}/artifacts/{version}/activate",
            post(activate_artifact),
        )
        .route(
            "/api/v1/agents/{id}/services/{name}/artifacts/{version}",
            axum::routing::delete(delete_artifact),
        )
        .route(
            "/api/v1/agents/{id}/services/{name}/publish",
            post(publish_service),
        )
        .route(
            "/api/v1/agents/{id}/services/{name}/rollback",
            post(rollback_service),
        )
        .route("/api/v1/proxies", get(list_proxies).post(register_proxy))
        .route(
            "/api/v1/proxies/{id}",
            get(get_proxy).put(update_proxy).delete(delete_proxy),
        )
        .route("/api/v1/proxies/{id}/ping", get(ping_proxy))
        .route(
            "/api/v1/proxies/{id}/rotate-token",
            post(rotate_proxy_token),
        )
        .route(
            "/api/v1/proxies/{id}/upstreams",
            get(list_proxy_upstreams).post(create_proxy_upstream),
        )
        .route(
            "/api/v1/proxies/{id}/upstreams/{upstream_id}",
            axum::routing::put(update_proxy_upstream).delete(delete_proxy_upstream),
        )
        .route("/api/v1/proxies/{id}/import", post(import_upstream))
        .layer(DefaultBodyLimit::max(ARTIFACT_BODY_LIMIT));

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

fn audit(
    state: &AppState,
    ctx: &AuthContext,
    action: &str,
    resource_type: &str,
    resource_id: &str,
    detail: impl Into<String>,
) {
    if let Err(e) = state.audit.append(
        ctx.user_id.as_deref(),
        &ctx.username,
        action,
        resource_type,
        resource_id,
        detail,
    ) {
        tracing::warn!(error = %e, "audit append failed");
    }
}

fn require_admin(auth: &AuthUser) -> Result<(), (StatusCode, Json<ApiResponse<()>>)> {
    if auth.0.is_admin() {
        Ok(())
    } else {
        Err((
            StatusCode::FORBIDDEN,
            Json(ApiResponse::<()>::err("admin role required")),
        ))
    }
}

fn user_err(e: UserError) -> (StatusCode, Json<ApiResponse<()>>) {
    let status = match &e {
        UserError::NotFound => StatusCode::NOT_FOUND,
        UserError::Duplicate => StatusCode::CONFLICT,
        UserError::InvalidCredentials => StatusCode::UNAUTHORIZED,
        UserError::Disabled => StatusCode::FORBIDDEN,
        UserError::Forbidden => StatusCode::FORBIDDEN,
        UserError::Other(_) => StatusCode::BAD_REQUEST,
        UserError::Db(_) => StatusCode::INTERNAL_SERVER_ERROR,
    };
    (status, Json(ApiResponse::<()>::err(e.to_string())))
}

#[derive(serde::Serialize)]
struct MeView {
    user_id: Option<String>,
    username: String,
    role: String,
    is_service: bool,
}

async fn login(
    State(state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<ApiResponse<LoginResponse>>, (StatusCode, Json<ApiResponse<()>>)> {
    match state.users.login(req) {
        Ok(resp) => {
            audit(
                &state,
                &AuthContext {
                    user_id: Some(resp.user.id.clone()),
                    username: resp.user.username.clone(),
                    role: resp.user.role.clone(),
                    is_service: false,
                    session_token: None,
                },
                "auth.login",
                "user",
                &resp.user.id,
                "login success",
            );
            Ok(Json(ApiResponse::ok("ok", resp)))
        }
        Err(e) => Err(user_err(e)),
    }
}

async fn logout(
    auth: AuthUser,
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<()>>, (StatusCode, Json<ApiResponse<()>>)> {
    if let Some(token) = auth.0.session_token.as_deref() {
        let _ = state.users.logout(token);
    }
    audit(&state, &auth.0, "auth.logout", "user", "", "logout");
    Ok(Json(ApiResponse::ok("ok", ())))
}

async fn auth_me(auth: AuthUser) -> Json<ApiResponse<MeView>> {
    Json(ApiResponse::ok(
        "ok",
        MeView {
            user_id: auth.0.user_id.clone(),
            username: auth.0.username.clone(),
            role: auth.0.role.clone(),
            is_service: auth.0.is_service,
        },
    ))
}

async fn change_password(
    auth: AuthUser,
    State(state): State<AppState>,
    Json(req): Json<ChangePasswordRequest>,
) -> Result<Json<ApiResponse<()>>, (StatusCode, Json<ApiResponse<()>>)> {
    let Some(uid) = auth.0.user_id.as_deref() else {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::<()>::err(
                "service token cannot change password; use a user session",
            )),
        ));
    };
    state.users.change_password(uid, req).map_err(user_err)?;
    audit(
        &state,
        &auth.0,
        "auth.change_password",
        "user",
        uid,
        "password changed",
    );
    Ok(Json(ApiResponse::ok("ok", ())))
}

async fn list_users(
    auth: AuthUser,
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<Vec<UserInfo>>>, (StatusCode, Json<ApiResponse<()>>)> {
    require_admin(&auth)?;
    match state.users.list_users() {
        Ok(list) => Ok(Json(ApiResponse::ok("ok", list))),
        Err(e) => Err(user_err(e)),
    }
}

async fn create_user(
    auth: AuthUser,
    State(state): State<AppState>,
    Json(req): Json<CreateUserRequest>,
) -> Result<Json<ApiResponse<UserInfo>>, (StatusCode, Json<ApiResponse<()>>)> {
    require_admin(&auth)?;
    let user = state.users.create_user(req).map_err(user_err)?;
    audit(
        &state,
        &auth.0,
        "user.create",
        "user",
        &user.id,
        format!("created {}", user.username),
    );
    Ok(Json(ApiResponse::ok("created", user)))
}

async fn set_user_disabled(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<SetDisabledRequest>,
) -> Result<Json<ApiResponse<UserInfo>>, (StatusCode, Json<ApiResponse<()>>)> {
    require_admin(&auth)?;
    if auth.0.user_id.as_deref() == Some(id.as_str()) && req.disabled {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::<()>::err("cannot disable yourself")),
        ));
    }
    let user = state.users.set_disabled(&id, req.disabled).map_err(user_err)?;
    audit(
        &state,
        &auth.0,
        if req.disabled {
            "user.disable"
        } else {
            "user.enable"
        },
        "user",
        &id,
        format!("{} disabled={}", user.username, req.disabled),
    );
    Ok(Json(ApiResponse::ok("ok", user)))
}

#[derive(Debug, Deserialize)]
struct AuditQuery {
    #[serde(default = "default_audit_limit")]
    limit: usize,
}

fn default_audit_limit() -> usize {
    100
}

async fn list_audit_logs(
    auth: AuthUser,
    State(state): State<AppState>,
    Query(q): Query<AuditQuery>,
) -> Result<Json<ApiResponse<Vec<crate::audit::AuditEntry>>>, (StatusCode, Json<ApiResponse<()>>)> {
    require_admin(&auth)?;
    match state.audit.list(q.limit) {
        Ok(list) => Ok(Json(ApiResponse::ok("ok", list))),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<()>::err(e.to_string())),
        )),
    }
}

async fn list_agents(
    _auth: AuthUser,
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<Vec<AgentInfo>>>, (StatusCode, Json<ApiResponse<()>>)> {
    match state.agents.list() {
        Ok(list) => Ok(Json(ApiResponse::ok("ok", mask_tokens(list)))),
        Err(e) => Err(registry_err(e)),
    }
}

async fn get_agent(
    _auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<AgentInfo>>, (StatusCode, Json<ApiResponse<()>>)> {
    match state.agents.get(&id) {
        Ok(agent) => Ok(Json(ApiResponse::ok("ok", mask_token(agent)))),
        Err(e) => Err(registry_err(e)),
    }
}

async fn register_agent(
    auth: AuthUser,
    State(state): State<AppState>,
    Json(req): Json<RegisterAgentRequest>,
) -> Result<Json<ApiResponse<AgentInfo>>, (StatusCode, Json<ApiResponse<()>>)> {
    match state.agents.register(req) {
        Ok(agent) => {
        audit(&state, &auth.0, "agent.register", "agent", &agent.id, &agent.name);
        Ok(Json(ApiResponse::ok("registered", mask_token(agent))))
    }
        Err(e) => Err(registry_err(e)),
    }
}

async fn update_agent(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<UpdateAgentRequest>,
) -> Result<Json<ApiResponse<AgentInfo>>, (StatusCode, Json<ApiResponse<()>>)> {
    match state.agents.update(&id, req) {
        Ok(agent) => {
        audit(&state, &auth.0, "agent.update", "agent", &agent.id, &agent.name);
        Ok(Json(ApiResponse::ok("updated", mask_token(agent))))
    }
        Err(e) => Err(registry_err(e)),
    }
}

async fn delete_agent(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<AgentInfo>>, (StatusCode, Json<ApiResponse<()>>)> {
    match state.agents.remove(&id) {
        Ok(agent) => {
        audit(&state, &auth.0, "agent.delete", "agent", &agent.id, &agent.name);
        Ok(Json(ApiResponse::ok("deleted", mask_token(agent))))
    }
        Err(e) => Err(registry_err(e)),
    }
}

async fn ping_agent(
    _auth: AuthUser,
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

fn new_token() -> String {
    format!("axle_{}", uuid::Uuid::new_v4().simple())
}

async fn overview_services(
    _auth: AuthUser,
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<OverviewResponse>>, (StatusCode, Json<ApiResponse<()>>)> {
    let agents = state.agents.list().map_err(registry_err)?;
    let agents_total = agents.len();
    let mut services = Vec::new();
    let mut agents_reachable = 0usize;

    let mut handles = Vec::with_capacity(agents.len());
    for agent in agents {
        let http = state.http.clone();
        handles.push(tokio::spawn(async move {
            let result = crate::proxy::agent_get(&http, &agent, "/api/v1/services").await;
            (agent, result)
        }));
    }

    for handle in handles {
        let Ok((agent, result)) = handle.await else {
            continue;
        };
        match result {
            Ok(v) => {
                agents_reachable += 1;
                let list = v
                    .get("data")
                    .cloned()
                    .unwrap_or(v);
                if let Some(arr) = list.as_array() {
                    for item in arr {
                        services.push(OverviewServiceRow {
                            agent_id: agent.id.clone(),
                            agent_name: agent.name.clone(),
                            agent_reachable: true,
                            name: item
                                .get("name")
                                .and_then(|x| x.as_str())
                                .unwrap_or("")
                                .to_string(),
                            state: item
                                .get("state")
                                .and_then(|x| x.as_str())
                                .unwrap_or("unknown")
                                .to_string(),
                            kind: item
                                .get("kind")
                                .and_then(|x| x.as_str())
                                .map(|s| s.to_string()),
                            pid: item.get("pid").and_then(|x| x.as_u64()),
                            target: item
                                .get("target")
                                .or_else(|| item.get("jar_path"))
                                .and_then(|x| x.as_str())
                                .map(|s| s.to_string()),
                            message: item
                                .get("message")
                                .and_then(|x| x.as_str())
                                .map(|s| s.to_string()),
                            healthy: item.get("healthy").and_then(|x| x.as_bool()),
                            error: None,
                        });
                    }
                } else {
                    services.push(OverviewServiceRow {
                        agent_id: agent.id.clone(),
                        agent_name: agent.name.clone(),
                        agent_reachable: true,
                        name: String::new(),
                        state: "unknown".into(),
                        kind: None,
                        pid: None,
                        target: None,
                        message: None,
                        healthy: None,
                        error: Some("unexpected services payload".into()),
                    });
                }
            }
            Err(e) => {
                services.push(OverviewServiceRow {
                    agent_id: agent.id.clone(),
                    agent_name: agent.name.clone(),
                    agent_reachable: false,
                    name: String::new(),
                    state: "unreachable".into(),
                    kind: None,
                    pid: None,
                    target: None,
                    message: None,
                    healthy: None,
                    error: Some(e.to_string()),
                });
            }
        }
    }

    Ok(Json(ApiResponse::ok(
        "ok",
        OverviewResponse {
            agents_total,
            agents_reachable,
            services,
        },
    )))
}

async fn rotate_agent_token(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<RotateTokenRequest>,
) -> Result<Json<ApiResponse<RotateTokenResponse>>, (StatusCode, Json<ApiResponse<()>>)> {
    let agent = state.agents.get(&id).map_err(registry_err)?;
    let mut token = new_token();
    let mut synced = false;
    let message = if req.sync {
        match crate::proxy::agent_post_json(
            &state.http,
            &agent,
            "/api/v1/auth/rotate-token",
            &serde_json::json!({}),
        )
        .await
        {
            Ok(v) => {
                if let Some(t) = v
                    .pointer("/data/token")
                    .and_then(|x| x.as_str())
                    .map(|s| s.to_string())
                {
                    token = t;
                    synced = true;
                    Some("remote agent token rotated".into())
                } else {
                    Some(
                        "agent responded but token missing; registry updated with new local token"
                            .into(),
                    )
                }
            }
            Err(e) => Some(format!(
                "sync failed ({e}); registry token updated — copy into agent config"
            )),
        }
    } else {
        Some("registry token updated — copy into agent/proxy if needed".into())
    };

    let updated = state
        .agents
        .update(
            &id,
            UpdateAgentRequest {
                name: None,
                base_url: None,
                token: Some(token.clone()),
                tags: None,
                proxy_id: None,
            },
        )
        .map_err(registry_err)?;

    audit(
        &state,
        &auth.0,
        "agent.rotate_token",
        "agent",
        &updated.id,
        if synced { "synced" } else { "local" },
    );

    Ok(Json(ApiResponse::ok(
        "rotated",
        RotateTokenResponse {
            id: updated.id,
            token,
            synced,
            message,
        },
    )))
}

async fn rotate_proxy_token(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(_req): Json<RotateTokenRequest>,
) -> Result<Json<ApiResponse<RotateTokenResponse>>, (StatusCode, Json<ApiResponse<()>>)> {
    let token = new_token();
    let updated = state
        .proxies
        .update(
            &id,
            UpdateProxyRequest {
                name: None,
                base_url: None,
                token: Some(token.clone()),
                notes: None,
            },
        )
        .map_err(proxy_registry_err)?;

    audit(
        &state,
        &auth.0,
        "proxy.rotate_token",
        "proxy",
        &updated.id,
        "local",
    );

    Ok(Json(ApiResponse::ok(
        "rotated",
        RotateTokenResponse {
            id: updated.id,
            token,
            synced: false,
            message: Some(
                "proxy registry token updated — update proxy config.toml and reload".into(),
            ),
        },
    )))
}

async fn list_services(
    _auth: AuthUser,
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
    auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<StartServiceRequest>,
) -> Result<Json<ApiResponse<Value>>, (StatusCode, Json<ApiResponse<()>>)> {
    let agent = state.agents.get(&id).map_err(registry_err)?;
    match crate::proxy::agent_post_json(&state.http, &agent, "/api/v1/services/start", &body)
        .await
    {
        Ok(v) => {
            audit(&state, &auth.0, "service.start", "agent", &id, format!("inline start {}", body.name));
            Ok(Json(ApiResponse::ok("ok", v)))
        },
        Err(e) => Err(proxy_err(e)),
    }
}

async fn save_service(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<StartServiceRequest>,
) -> Result<Json<ApiResponse<Value>>, (StatusCode, Json<ApiResponse<()>>)> {
    let agent = state.agents.get(&id).map_err(registry_err)?;
    match crate::proxy::agent_post_json(&state.http, &agent, "/api/v1/services", &body).await {
        Ok(v) => {
            audit(&state, &auth.0, "service.save", "agent", &id, format!("save {}", body.name));
            Ok(Json(ApiResponse::ok("ok", v)))
        },
        Err(e) => Err(proxy_err(e)),
    }
}

async fn save_service_named(
    auth: AuthUser,
    State(state): State<AppState>,
    Path((id, name)): Path<(String, String)>,
    Json(mut body): Json<StartServiceRequest>,
) -> Result<Json<ApiResponse<Value>>, (StatusCode, Json<ApiResponse<()>>)> {
    body.name = name.clone();
    let agent = state.agents.get(&id).map_err(registry_err)?;
    let path = format!("/api/v1/services/{name}");
    match crate::proxy::agent_put_json(&state.http, &agent, &path, &body).await {
        Ok(v) => {
            audit(&state, &auth.0, "service.save", "agent", &id, format!("save {name}"));
            Ok(Json(ApiResponse::ok("ok", v)))
        },
        Err(e) => Err(proxy_err(e)),
    }
}

async fn get_service_spec(
    _auth: AuthUser,
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
    auth: AuthUser,
    State(state): State<AppState>,
    Path((id, name)): Path<(String, String)>,
) -> Result<Json<ApiResponse<Value>>, (StatusCode, Json<ApiResponse<()>>)> {
    let agent = state.agents.get(&id).map_err(registry_err)?;
    let path = format!("/api/v1/services/{name}/start");
    match crate::proxy::agent_post_empty(&state.http, &agent, &path).await {
        Ok(v) => {
            audit(&state, &auth.0, "service.start", "agent", &id, format!("start {name}"));
            Ok(Json(ApiResponse::ok("ok", v)))
        },
        Err(e) => Err(proxy_err(e)),
    }
}

async fn restart_service(
    auth: AuthUser,
    State(state): State<AppState>,
    Path((id, name)): Path<(String, String)>,
) -> Result<Json<ApiResponse<Value>>, (StatusCode, Json<ApiResponse<()>>)> {
    let agent = state.agents.get(&id).map_err(registry_err)?;
    let path = format!("/api/v1/services/{name}/restart");
    match crate::proxy::agent_post_empty(&state.http, &agent, &path).await {
        Ok(v) => {
            audit(&state, &auth.0, "service.restart", "agent", &id, format!("restart {name}"));
            Ok(Json(ApiResponse::ok("ok", v)))
        },
        Err(e) => Err(proxy_err(e)),
    }
}

async fn remove_service(
    auth: AuthUser,
    State(state): State<AppState>,
    Path((id, name)): Path<(String, String)>,
) -> Result<Json<ApiResponse<Value>>, (StatusCode, Json<ApiResponse<()>>)> {
    let agent = state.agents.get(&id).map_err(registry_err)?;
    let path = format!("/api/v1/services/{name}");
    match crate::proxy::agent_delete(&state.http, &agent, &path).await {
        Ok(v) => {
            audit(&state, &auth.0, "service.delete", "agent", &id, format!("delete {name}"));
            Ok(Json(ApiResponse::ok("ok", v)))
        },
        Err(e) => Err(proxy_err(e)),
    }
}

async fn service_status(
    _auth: AuthUser,
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
    auth: AuthUser,
    State(state): State<AppState>,
    Path((id, name)): Path<(String, String)>,
) -> Result<Json<ApiResponse<Value>>, (StatusCode, Json<ApiResponse<()>>)> {
    let agent = state.agents.get(&id).map_err(registry_err)?;
    let path = format!("/api/v1/services/{name}/stop");
    match crate::proxy::agent_post_empty(&state.http, &agent, &path).await {
        Ok(v) => {
            audit(&state, &auth.0, "service.stop", "agent", &id, format!("stop {name}"));
            Ok(Json(ApiResponse::ok("ok", v)))
        },
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
    _auth: AuthUser,
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

async fn list_artifacts(
    _auth: AuthUser,
    State(state): State<AppState>,
    Path((id, name)): Path<(String, String)>,
) -> Result<Json<ApiResponse<Value>>, (StatusCode, Json<ApiResponse<()>>)> {
    let agent = state.agents.get(&id).map_err(registry_err)?;
    let path = format!("/api/v1/services/{name}/artifacts");
    match crate::proxy::agent_get(&state.http, &agent, &path).await {
        Ok(v) => Ok(Json(ApiResponse::ok("ok", v))),
        Err(e) => Err(proxy_err(e)),
    }
}

async fn upload_artifact(
    auth: AuthUser,
    State(state): State<AppState>,
    Path((id, name)): Path<(String, String)>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Json<ApiResponse<Value>>, (StatusCode, Json<ApiResponse<()>>)> {
    let agent = state.agents.get(&id).map_err(registry_err)?;
    let ct = headers
        .get(axum::http::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("application/octet-stream");
    let path = format!("/api/v1/services/{name}/artifacts");
    match crate::proxy::agent_post_raw(&state.http, &agent, &path, body.to_vec(), ct).await {
        Ok(v) => {
            audit(
                &state,
                &auth.0,
                "service.artifact_upload",
                "agent",
                &id,
                format!("upload artifact for {name}"),
            );
            Ok(Json(ApiResponse::ok("ok", v)))
        }
        Err(e) => Err(proxy_err(e)),
    }
}

#[derive(Debug, Deserialize, Serialize)]
struct PublishBody {
    version: String,
    #[serde(default = "default_true")]
    start: bool,
}

#[derive(Debug, Deserialize, Serialize)]
struct RollbackBody {
    #[serde(default = "default_true")]
    start: bool,
}

fn default_true() -> bool {
    true
}

async fn activate_artifact(
    auth: AuthUser,
    State(state): State<AppState>,
    Path((id, name, version)): Path<(String, String, String)>,
) -> Result<Json<ApiResponse<Value>>, (StatusCode, Json<ApiResponse<()>>)> {
    let agent = state.agents.get(&id).map_err(registry_err)?;
    let path = format!("/api/v1/services/{name}/artifacts/{version}/activate");
    match crate::proxy::agent_post_empty(&state.http, &agent, &path).await {
        Ok(v) => {
            audit(
                &state,
                &auth.0,
                "service.artifact_activate",
                "agent",
                &id,
                format!("activate {version} for {name}"),
            );
            Ok(Json(ApiResponse::ok("ok", v)))
        }
        Err(e) => Err(proxy_err(e)),
    }
}

async fn delete_artifact(
    auth: AuthUser,
    State(state): State<AppState>,
    Path((id, name, version)): Path<(String, String, String)>,
) -> Result<Json<ApiResponse<Value>>, (StatusCode, Json<ApiResponse<()>>)> {
    let agent = state.agents.get(&id).map_err(registry_err)?;
    let path = format!("/api/v1/services/{name}/artifacts/{version}");
    match crate::proxy::agent_delete(&state.http, &agent, &path).await {
        Ok(v) => {
            audit(
                &state,
                &auth.0,
                "service.artifact_delete",
                "agent",
                &id,
                format!("delete {version} for {name}"),
            );
            Ok(Json(ApiResponse::ok("ok", v)))
        }
        Err(e) => Err(proxy_err(e)),
    }
}

async fn publish_service(
    auth: AuthUser,
    State(state): State<AppState>,
    Path((id, name)): Path<(String, String)>,
    Json(body): Json<PublishBody>,
) -> Result<Json<ApiResponse<Value>>, (StatusCode, Json<ApiResponse<()>>)> {
    let agent = state.agents.get(&id).map_err(registry_err)?;
    let path = format!("/api/v1/services/{name}/publish");
    match crate::proxy::agent_post_json(&state.http, &agent, &path, &body).await {
        Ok(v) => {
            audit(
                &state,
                &auth.0,
                "service.publish",
                "agent",
                &id,
                format!("publish {} for {name}", body.version),
            );
            Ok(Json(ApiResponse::ok("ok", v)))
        }
        Err(e) => Err(proxy_err(e)),
    }
}

async fn rollback_service(
    auth: AuthUser,
    State(state): State<AppState>,
    Path((id, name)): Path<(String, String)>,
    body: Option<Json<RollbackBody>>,
) -> Result<Json<ApiResponse<Value>>, (StatusCode, Json<ApiResponse<()>>)> {
    let agent = state.agents.get(&id).map_err(registry_err)?;
    let path = format!("/api/v1/services/{name}/rollback");
    let payload = body.map(|b| b.0).unwrap_or(RollbackBody { start: true });
    match crate::proxy::agent_post_json(&state.http, &agent, &path, &payload).await {
        Ok(v) => {
            audit(
                &state,
                &auth.0,
                "service.rollback",
                "agent",
                &id,
                format!("rollback {name}"),
            );
            Ok(Json(ApiResponse::ok("ok", v)))
        }
        Err(e) => Err(proxy_err(e)),
    }
}

async fn list_proxies(
    _auth: AuthUser,
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<Vec<ProxyInfo>>>, (StatusCode, Json<ApiResponse<()>>)> {
    match state.proxies.list() {
        Ok(list) => Ok(Json(ApiResponse::ok("ok", mask_proxy_tokens(list)))),
        Err(e) => Err(proxy_registry_err(e)),
    }
}

async fn get_proxy(
    _auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<ProxyInfo>>, (StatusCode, Json<ApiResponse<()>>)> {
    match state.proxies.get(&id) {
        Ok(p) => Ok(Json(ApiResponse::ok("ok", mask_proxy_token(p)))),
        Err(e) => Err(proxy_registry_err(e)),
    }
}

async fn register_proxy(
    auth: AuthUser,
    State(state): State<AppState>,
    Json(req): Json<RegisterProxyRequest>,
) -> Result<Json<ApiResponse<ProxyInfo>>, (StatusCode, Json<ApiResponse<()>>)> {
    match state.proxies.register(req) {
        Ok(p) => {
        audit(&state, &auth.0, "proxy.register", "proxy", &p.id, &p.name);
        Ok(Json(ApiResponse::ok("registered", mask_proxy_token(p))))
    }
        Err(e) => Err(proxy_registry_err(e)),
    }
}

async fn update_proxy(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<UpdateProxyRequest>,
) -> Result<Json<ApiResponse<ProxyInfo>>, (StatusCode, Json<ApiResponse<()>>)> {
    match state.proxies.update(&id, req) {
        Ok(p) => {
        audit(&state, &auth.0, "proxy.update", "proxy", &p.id, &p.name);
        Ok(Json(ApiResponse::ok("updated", mask_proxy_token(p))))
    }
        Err(e) => Err(proxy_registry_err(e)),
    }
}

async fn delete_proxy(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<ProxyInfo>>, (StatusCode, Json<ApiResponse<()>>)> {
    match state.proxies.remove(&id) {
        Ok(p) => {
        audit(&state, &auth.0, "proxy.delete", "proxy", &p.id, &p.name);
        Ok(Json(ApiResponse::ok("deleted", mask_proxy_token(p))))
    }
        Err(e) => Err(proxy_registry_err(e)),
    }
}

fn proxy_as_agent(p: &ProxyInfo) -> AgentInfo {
    AgentInfo {
        id: p.id.clone(),
        name: p.name.clone(),
        base_url: p.base_url.clone(),
        token: p.token.clone(),
        tags: vec![],
        proxy_id: None,
        created_at: p.created_at,
        updated_at: p.updated_at,
    }
}

async fn ping_proxy(
    _auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<ProxyHealthView>>, (StatusCode, Json<ApiResponse<()>>)> {
    let proxy = state.proxies.get(&id).map_err(proxy_registry_err)?;
    let as_agent = proxy_as_agent(&proxy);
    match crate::proxy::agent_health(&state.http, &as_agent).await {
        Ok(detail) => Ok(Json(ApiResponse::ok(
            "reachable",
            ProxyHealthView {
                proxy_id: proxy.id,
                proxy_name: proxy.name,
                reachable: true,
                detail,
            },
        ))),
        Err(e) => Ok(Json(ApiResponse::ok(
            "unreachable",
            ProxyHealthView {
                proxy_id: proxy.id,
                proxy_name: proxy.name,
                reachable: false,
                detail: Value::String(e.to_string()),
            },
        ))),
    }
}

async fn list_proxy_upstreams(
    _auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<Vec<UpstreamView>>>, (StatusCode, Json<ApiResponse<()>>)> {
    let proxy = state.proxies.get(&id).map_err(proxy_registry_err)?;
    let as_agent = proxy_as_agent(&proxy);
    let v = crate::proxy::agent_get(&state.http, &as_agent, "/api/v1/upstreams")
        .await
        .map_err(proxy_err)?;
    let list = parse_upstreams(&v).map_err(|e| {
        (
            StatusCode::BAD_GATEWAY,
            Json(ApiResponse::<()>::err(e)),
        )
    })?;
    Ok(Json(ApiResponse::ok("ok", list)))
}

async fn create_proxy_upstream(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<CreateUpstreamRequest>,
) -> Result<Json<ApiResponse<UpstreamView>>, (StatusCode, Json<ApiResponse<()>>)> {
    let proxy = state.proxies.get(&id).map_err(proxy_registry_err)?;
    let as_agent = proxy_as_agent(&proxy);
    let v = crate::proxy::agent_post_json(&state.http, &as_agent, "/api/v1/upstreams", &body)
        .await
        .map_err(proxy_err)?;
    let view = parse_upstream_one(&v).map_err(|e| {
        (
            StatusCode::BAD_GATEWAY,
            Json(ApiResponse::<()>::err(e)),
        )
    })?;
    audit(
        &state,
        &auth.0,
        "upstream.create",
        "proxy",
        &id,
        format!("create {}", body.id),
    );
    Ok(Json(ApiResponse::ok("created", view)))
}

async fn update_proxy_upstream(
    auth: AuthUser,
    State(state): State<AppState>,
    Path((id, upstream_id)): Path<(String, String)>,
    Json(body): Json<UpdateUpstreamRequest>,
) -> Result<Json<ApiResponse<UpstreamView>>, (StatusCode, Json<ApiResponse<()>>)> {
    let proxy = state.proxies.get(&id).map_err(proxy_registry_err)?;
    let as_agent = proxy_as_agent(&proxy);
    let path = format!("/api/v1/upstreams/{upstream_id}");
    let v = crate::proxy::agent_put_json(&state.http, &as_agent, &path, &body)
        .await
        .map_err(proxy_err)?;
    let view = parse_upstream_one(&v).map_err(|e| {
        (
            StatusCode::BAD_GATEWAY,
            Json(ApiResponse::<()>::err(e)),
        )
    })?;
    audit(
        &state,
        &auth.0,
        "upstream.update",
        "proxy",
        &id,
        format!("update {upstream_id}"),
    );
    Ok(Json(ApiResponse::ok("updated", view)))
}

async fn delete_proxy_upstream(
    auth: AuthUser,
    State(state): State<AppState>,
    Path((id, upstream_id)): Path<(String, String)>,
) -> Result<Json<ApiResponse<UpstreamView>>, (StatusCode, Json<ApiResponse<()>>)> {
    let proxy = state.proxies.get(&id).map_err(proxy_registry_err)?;
    let as_agent = proxy_as_agent(&proxy);
    let path = format!("/api/v1/upstreams/{upstream_id}");
    let v = crate::proxy::agent_delete(&state.http, &as_agent, &path)
        .await
        .map_err(proxy_err)?;
    let view = parse_upstream_one(&v).map_err(|e| {
        (
            StatusCode::BAD_GATEWAY,
            Json(ApiResponse::<()>::err(e)),
        )
    })?;
    audit(
        &state,
        &auth.0,
        "upstream.delete",
        "proxy",
        &id,
        format!("delete {upstream_id}"),
    );
    Ok(Json(ApiResponse::ok("deleted", view)))
}

async fn import_upstream(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<ImportUpstreamRequest>,
) -> Result<Json<ApiResponse<AgentInfo>>, (StatusCode, Json<ApiResponse<()>>)> {
    let proxy = state.proxies.get(&id).map_err(proxy_registry_err)?;
    let as_agent = proxy_as_agent(&proxy);
    let v = crate::proxy::agent_get(&state.http, &as_agent, "/api/v1/upstreams")
        .await
        .map_err(proxy_err)?;
    let list = parse_upstreams(&v).map_err(|e| {
        (
            StatusCode::BAD_GATEWAY,
            Json(ApiResponse::<()>::err(e)),
        )
    })?;
    let upstream_id = req.upstream_id.trim();
    let Some(up) = list.iter().find(|u| u.id == upstream_id) else {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ApiResponse::<()>::err(format!(
                "upstream `{upstream_id}` not found on proxy"
            ))),
        ));
    };

    let agent_name = req
        .agent_name
        .as_ref()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| up.name.clone());

    let mut tags = req.tags;
    let via = format!("via:{}", proxy.name);
    if !tags.iter().any(|t| t == &via) {
        tags.push(via);
    }

    let base_url = format!(
        "{}{}",
        proxy.base_url.trim_end_matches('/'),
        if up.path_prefix.starts_with('/') {
            up.path_prefix.clone()
        } else {
            format!("/{}", up.path_prefix)
        }
    );

    let agent_req = RegisterAgentRequest {
        name: agent_name,
        base_url,
        token: proxy.token.clone(),
        tags,
        proxy_id: Some(proxy.id),
    };

    match state.agents.register(agent_req) {
        Ok(agent) => {
        audit(&state, &auth.0, "agent.import", "agent", &agent.id, format!("via proxy {id}"));
        Ok(Json(ApiResponse::ok("imported", mask_token(agent))))
    }
        Err(e) => Err(registry_err(e)),
    }
}

fn parse_upstreams(v: &Value) -> Result<Vec<UpstreamView>, String> {
    // axleops-proxy returns { ok, message, data: [ {id,name,path_prefix,...} ] }
    let data = v.get("data").cloned().unwrap_or_else(|| v.clone());
    if data.is_null() {
        return Ok(vec![]);
    }
    serde_json::from_value(data).map_err(|e| format!("invalid upstreams payload: {e}"))
}

fn parse_upstream_one(v: &Value) -> Result<UpstreamView, String> {
    let data = v.get("data").cloned().unwrap_or_else(|| v.clone());
    serde_json::from_value(data).map_err(|e| format!("invalid upstream payload: {e}"))
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

fn mask_proxy_token(mut p: ProxyInfo) -> ProxyInfo {
    if p.token.len() > 4 {
        p.token = format!("{}****", &p.token[..4]);
    } else {
        p.token = "****".into();
    }
    p
}

fn mask_proxy_tokens(list: Vec<ProxyInfo>) -> Vec<ProxyInfo> {
    list.into_iter().map(mask_proxy_token).collect()
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

fn proxy_registry_err(e: ProxyRegistryError) -> (StatusCode, Json<ApiResponse<()>>) {
    let status = match &e {
        ProxyRegistryError::NotFound => StatusCode::NOT_FOUND,
        ProxyRegistryError::DuplicateName(_) => StatusCode::CONFLICT,
        ProxyRegistryError::Other(_) => StatusCode::BAD_REQUEST,
        ProxyRegistryError::Db(_) => StatusCode::INTERNAL_SERVER_ERROR,
    };
    (status, Json(ApiResponse::<()>::err(e.to_string())))
}

fn proxy_err(e: ProxyError) -> (StatusCode, Json<ApiResponse<()>>) {
    let (status, message) = match &e {
        ProxyError::AgentStatus { status, body } => {
            let code = StatusCode::from_u16(status.as_u16()).unwrap_or(StatusCode::BAD_GATEWAY);
            let hint = match status.as_u16() {
                401 | 403 => "?????Token ?????",
                404 => "???????? Agent/Proxy ????? id",
                409 => "?????????????",
                502 | 503 | 504 => "??????? Proxy ????? Agent ???",
                _ => "??????",
            };
            let body = body.trim();
            let detail = if body.is_empty() {
                String::new()
            } else if body.len() > 240 {
                format!(" ? {}?", &body[..240])
            } else {
                format!(" ? {body}")
            };
            (code, format!("{hint}?HTTP {status}?{detail}"))
        }
        ProxyError::Http(err) => {
            let msg = if err.is_timeout() {
                format!("????????????????{err}?")
            } else if err.is_connect() {
                format!("???? Agent/Proxy?????????????????{err}?")
            } else {
                format!("???????{err}")
            };
            (StatusCode::BAD_GATEWAY, msg)
        }
        ProxyError::Other(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg.clone()),
    };
    (status, Json(ApiResponse::<()>::err(message)))
}
