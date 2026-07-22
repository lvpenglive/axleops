use crate::auth::AuthToken;
use crate::forward::{forward_to_agent, join_path_query};
use crate::store::{StoreError, UpdateUpstreamRequest, UpsertUpstreamRequest, UpstreamRecord};
use crate::AppState;
use axum::body::Bytes;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, Method, StatusCode, Uri};
use axum::response::{IntoResponse, Response};
use axum::routing::{any, get};
use axum::{Json, Router};
use serde::Serialize;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/health", get(health))
        .route("/api/v1/upstreams", get(list_upstreams).post(create_upstream))
        .route(
            "/api/v1/upstreams/{id}",
            axum::routing::put(update_upstream).delete(delete_upstream),
        )
        .route("/a/{id}", any(forward_root))
        .route("/a/{id}/{*rest}", any(forward_rest))
}

async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "ok": true,
        "service": "axleops-proxy",
        "version": env!("CARGO_PKG_VERSION"),
    }))
}

#[derive(Serialize)]
struct UpstreamView {
    id: String,
    name: String,
    base_url: String,
    /// Public path prefix Admin should use as base_url (relative to this proxy).
    path_prefix: String,
}

fn to_view(r: &UpstreamRecord) -> UpstreamView {
    UpstreamView {
        id: r.id.clone(),
        name: r.name.clone(),
        base_url: r.base_url.clone(),
        path_prefix: format!("/a/{}", r.id),
    }
}

fn store_err(e: StoreError) -> (StatusCode, Json<serde_json::Value>) {
    let status = match &e {
        StoreError::NotFound => StatusCode::NOT_FOUND,
        StoreError::Duplicate(_) => StatusCode::CONFLICT,
        StoreError::Other(_) => StatusCode::BAD_REQUEST,
    };
    (
        status,
        Json(serde_json::json!({
            "ok": false,
            "message": e.to_string(),
        })),
    )
}

fn verify_err(message: impl Into<String>) -> (StatusCode, Json<serde_json::Value>) {
    (
        StatusCode::BAD_REQUEST,
        Json(serde_json::json!({
            "ok": false,
            "message": message.into(),
        })),
    )
}

/// Probe Agent with the given token (authenticated API). Distinguishes bad token vs unreachable.
async fn verify_agent_token(
    client: &reqwest::Client,
    base_url: &str,
    token: &str,
) -> Result<(), (StatusCode, Json<serde_json::Value>)> {
    let base = base_url.trim().trim_end_matches('/');
    if base.contains("/a/") || base.ends_with("/a") {
        return Err(verify_err(
            "下游 Base URL 应填 Agent 真实地址（例如 http://10.0.1.11:9100），不要填 Proxy 的 http://proxy:9200/a/<id>",
        ));
    }
    if token.trim().is_empty() {
        return Err(verify_err("Agent Token 不能为空"));
    }

    let url = format!("{base}/api/v1/services");
    let resp = client
        .get(&url)
        .header("X-AxleOps-Token", token.trim())
        .send()
        .await
        .map_err(|e| {
            verify_err(format!(
                "Proxy 无法连接 Agent（探测 {url}）。\
                 请填 Proxy 机器能访问的地址，不要用 Admin 本机的 127.0.0.1（除非 Agent 与 Proxy 同机）。{e}"
            ))
        })?;
    let status = resp.status();
    let body = resp.text().await.unwrap_or_default();
    if status.as_u16() == 401 || status.as_u16() == 403 {
        // Hitting Proxy itself with Agent token often yields Proxy-auth 401.
        if body.contains("Proxy token") || body.contains("Admin→Proxy") {
            return Err(verify_err(format!(
                "探测地址更像 Proxy 而不是 Agent（{url}）。下游 Base URL 请改为 Agent 的 host:9100。详情：{body}"
            )));
        }
        return Err(verify_err(format!(
            "下游 Agent Token 被拒绝（探测 {url}，token 长度 {}）。\
             请与正在运行的 Agent 一致：优先看 data/auth.token，否则才是 config.toml 的 token。详情：{body}",
            token.trim().chars().count()
        )));
    }
    if !status.is_success() {
        return Err(verify_err(format!(
            "下游 Agent 返回 HTTP {status}（探测 {url}）：{body}"
        )));
    }
    Ok(())
}

async fn list_upstreams(
    State(state): State<AppState>,
    _auth: AuthToken,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let list = state.store.list().map_err(store_err)?;
    let views: Vec<UpstreamView> = list.iter().map(to_view).collect();
    Ok(Json(serde_json::json!({
        "ok": true,
        "message": "upstreams",
        "data": views,
    })))
}

async fn create_upstream(
    State(state): State<AppState>,
    _auth: AuthToken,
    Json(req): Json<UpsertUpstreamRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let base_url = req.base_url.trim().trim_end_matches('/').to_string();
    let token = req.token.trim().to_string();
    verify_agent_token(&state.http, &base_url, &token).await?;

    let mut req = req;
    req.base_url = base_url;
    req.token = token;
    let rec = state.store.create(req).map_err(store_err)?;
    tracing::info!(id = %rec.id, base_url = %rec.base_url, "upstream created");
    Ok(Json(serde_json::json!({
        "ok": true,
        "message": "created",
        "data": to_view(&rec),
    })))
}

async fn update_upstream(
    State(state): State<AppState>,
    _auth: AuthToken,
    Path(id): Path<String>,
    Json(req): Json<UpdateUpstreamRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let existing = state.store.get(&id).map_err(store_err)?;
    let base_url = req
        .base_url
        .as_ref()
        .map(|s| s.trim().trim_end_matches('/').to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| existing.base_url.clone());
    let token = req
        .token
        .as_ref()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| existing.token.clone());
    verify_agent_token(&state.http, &base_url, &token).await?;

    let mut req = req;
    if req.base_url.is_some() {
        req.base_url = Some(base_url);
    }
    if req.token.is_some() {
        req.token = Some(token);
    }
    let rec = state.store.update(&id, req).map_err(store_err)?;
    tracing::info!(id = %rec.id, "upstream updated");
    Ok(Json(serde_json::json!({
        "ok": true,
        "message": "updated",
        "data": to_view(&rec),
    })))
}

async fn delete_upstream(
    State(state): State<AppState>,
    _auth: AuthToken,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let rec = state.store.remove(&id).map_err(store_err)?;
    tracing::info!(id = %rec.id, "upstream deleted");
    Ok(Json(serde_json::json!({
        "ok": true,
        "message": "deleted",
        "data": to_view(&rec),
    })))
}

async fn forward_root(
    State(state): State<AppState>,
    auth: AuthToken,
    Path(id): Path<String>,
    method: Method,
    uri: Uri,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let _ = auth;
    dispatch(&state, &id, None, method, uri, headers, body).await
}

async fn forward_rest(
    State(state): State<AppState>,
    auth: AuthToken,
    Path((id, rest)): Path<(String, String)>,
    method: Method,
    uri: Uri,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let _ = auth;
    dispatch(&state, &id, Some(rest.as_str()), method, uri, headers, body).await
}

async fn dispatch(
    state: &AppState,
    id: &str,
    rest: Option<&str>,
    method: Method,
    uri: Uri,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let upstream = match state.store.get(id) {
        Ok(u) => u,
        Err(StoreError::NotFound) => {
            return (
                StatusCode::NOT_FOUND,
                format!("unknown upstream id `{id}`"),
            )
                .into_response();
        }
        Err(e) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response();
        }
    };

    let target = join_path_query(rest, &uri);
    tracing::debug!(
        agent = %id,
        %method,
        target = %target,
        "forwarding"
    );

    forward_to_agent(&state.http, &upstream, method, &target, &headers, body).await
}
