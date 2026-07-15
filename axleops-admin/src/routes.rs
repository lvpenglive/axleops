use crate::auth::AuthToken;
use crate::models::{
    AgentHealthView, AgentInfo, ApiResponse, CreateUpstreamRequest, HealthInfo,
    ImportUpstreamRequest, ProxyHealthView, ProxyInfo, RegisterAgentRequest, RegisterProxyRequest,
    StartServiceRequest, UpdateAgentRequest, UpdateProxyRequest, UpdateUpstreamRequest,
    UpstreamView,
};
use crate::{agents::RegistryError, proxies::ProxyRegistryError, proxy::ProxyError, AppState};
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
        )
        .route("/api/v1/proxies", get(list_proxies).post(register_proxy))
        .route(
            "/api/v1/proxies/{id}",
            get(get_proxy).put(update_proxy).delete(delete_proxy),
        )
        .route("/api/v1/proxies/{id}/ping", get(ping_proxy))
        .route(
            "/api/v1/proxies/{id}/upstreams",
            get(list_proxy_upstreams).post(create_proxy_upstream),
        )
        .route(
            "/api/v1/proxies/{id}/upstreams/{upstream_id}",
            axum::routing::put(update_proxy_upstream).delete(delete_proxy_upstream),
        )
        .route("/api/v1/proxies/{id}/import", post(import_upstream));

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

async fn list_proxies(
    _auth: AuthToken,
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<Vec<ProxyInfo>>>, (StatusCode, Json<ApiResponse<()>>)> {
    match state.proxies.list() {
        Ok(list) => Ok(Json(ApiResponse::ok("ok", mask_proxy_tokens(list)))),
        Err(e) => Err(proxy_registry_err(e)),
    }
}

async fn get_proxy(
    _auth: AuthToken,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<ProxyInfo>>, (StatusCode, Json<ApiResponse<()>>)> {
    match state.proxies.get(&id) {
        Ok(p) => Ok(Json(ApiResponse::ok("ok", mask_proxy_token(p)))),
        Err(e) => Err(proxy_registry_err(e)),
    }
}

async fn register_proxy(
    _auth: AuthToken,
    State(state): State<AppState>,
    Json(req): Json<RegisterProxyRequest>,
) -> Result<Json<ApiResponse<ProxyInfo>>, (StatusCode, Json<ApiResponse<()>>)> {
    match state.proxies.register(req) {
        Ok(p) => Ok(Json(ApiResponse::ok("registered", mask_proxy_token(p)))),
        Err(e) => Err(proxy_registry_err(e)),
    }
}

async fn update_proxy(
    _auth: AuthToken,
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<UpdateProxyRequest>,
) -> Result<Json<ApiResponse<ProxyInfo>>, (StatusCode, Json<ApiResponse<()>>)> {
    match state.proxies.update(&id, req) {
        Ok(p) => Ok(Json(ApiResponse::ok("updated", mask_proxy_token(p)))),
        Err(e) => Err(proxy_registry_err(e)),
    }
}

async fn delete_proxy(
    _auth: AuthToken,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<ProxyInfo>>, (StatusCode, Json<ApiResponse<()>>)> {
    match state.proxies.remove(&id) {
        Ok(p) => Ok(Json(ApiResponse::ok("deleted", mask_proxy_token(p)))),
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
    _auth: AuthToken,
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
    _auth: AuthToken,
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
    _auth: AuthToken,
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
    Ok(Json(ApiResponse::ok("created", view)))
}

async fn update_proxy_upstream(
    _auth: AuthToken,
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
    Ok(Json(ApiResponse::ok("updated", view)))
}

async fn delete_proxy_upstream(
    _auth: AuthToken,
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
    Ok(Json(ApiResponse::ok("deleted", view)))
}

async fn import_upstream(
    _auth: AuthToken,
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
        Ok(agent) => Ok(Json(ApiResponse::ok("imported", mask_token(agent)))),
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
                401 | 403 => "认证失败：Token 可能不正确",
                404 => "目标不存在：检查 Agent/Proxy 路径或上游 id",
                409 => "冲突：资源状态不允许该操作",
                502 | 503 | 504 => "下游不可用：经 Proxy 转发失败或 Agent 无响应",
                _ => "目标返回错误",
            };
            let body = body.trim();
            let detail = if body.is_empty() {
                String::new()
            } else if body.len() > 240 {
                format!(" — {}…", &body[..240])
            } else {
                format!(" — {body}")
            };
            (
                code,
                format!("{hint}（HTTP {status}）{detail}"),
            )
        }
        ProxyError::Http(err) => {
            let msg = if err.is_timeout() {
                format!("请求超时：目标无响应或网络过慢（{err}）")
            } else if err.is_connect() {
                format!("无法连接 Agent/Proxy：检查地址、防火墙与进程是否启动（{err}）")
            } else {
                format!("转发请求失败：{err}")
            };
            (StatusCode::BAD_GATEWAY, msg)
        }
        ProxyError::Other(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg.clone()),
    };
    (status, Json(ApiResponse::<()>::err(message)))
}
