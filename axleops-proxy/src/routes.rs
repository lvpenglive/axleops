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
