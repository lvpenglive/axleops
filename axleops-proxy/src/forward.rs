use crate::config::Upstream;
use axum::body::Body;
use axum::http::{HeaderMap, HeaderName, HeaderValue, Method, StatusCode, Uri};
use axum::response::{IntoResponse, Response};
use bytes::Bytes;
use reqwest::Client;

const HOP_BY_HOP: &[&str] = &[
    "connection",
    "keep-alive",
    "proxy-authenticate",
    "proxy-authorization",
    "te",
    "trailers",
    "transfer-encoding",
    "upgrade",
    "host",
    "content-length",
];

pub async fn forward_to_agent(
    client: &Client,
    upstream: &Upstream,
    method: Method,
    // Path+query after /a/{id}, e.g. "/api/v1/services?x=1" or "/health"
    target_path_and_query: &str,
    inbound_headers: &HeaderMap,
    body: Bytes,
) -> Response {
    let path = if target_path_and_query.is_empty() || target_path_and_query == "/" {
        "/"
    } else if target_path_and_query.starts_with('/') {
        target_path_and_query
    } else {
        // shouldn't happen; normalize
        return (
            StatusCode::BAD_REQUEST,
            "forward path must start with /",
        )
            .into_response();
    };

    let url = format!("{}{}", upstream.base_url, path);
    let reqwest_method = match method.as_str() {
        "GET" => reqwest::Method::GET,
        "POST" => reqwest::Method::POST,
        "PUT" => reqwest::Method::PUT,
        "DELETE" => reqwest::Method::DELETE,
        "PATCH" => reqwest::Method::PATCH,
        "HEAD" => reqwest::Method::HEAD,
        "OPTIONS" => reqwest::Method::OPTIONS,
        other => {
            return (StatusCode::METHOD_NOT_ALLOWED, format!("unsupported method {other}"))
                .into_response();
        }
    };

    let mut builder = client.request(reqwest_method, &url);
    builder = builder.header("X-AxleOps-Token", upstream.token.trim());

    for (name, value) in inbound_headers.iter() {
        let key = name.as_str();
        if HOP_BY_HOP.iter().any(|h| h.eq_ignore_ascii_case(key)) {
            continue;
        }
        if key.eq_ignore_ascii_case("x-axleops-token") {
            continue; // replaced with agent token
        }
        if let Ok(v) = HeaderValue::from_bytes(value.as_bytes()) {
            builder = builder.header(name.clone(), v);
        }
    }

    if !body.is_empty() {
        builder = builder.body(body);
    }

    match builder.send().await {
        Ok(resp) => match response_from_reqwest(resp).await {
            Ok(r) => r,
            Err(e) => (
                StatusCode::BAD_GATEWAY,
                format!("failed to read agent response: {e}"),
            )
                .into_response(),
        },
        Err(e) => {
            tracing::warn!(agent = %upstream.id, url = %url, error = %e, "upstream request failed");
            (StatusCode::BAD_GATEWAY, format!("agent unreachable: {e}")).into_response()
        }
    }
}

async fn response_from_reqwest(resp: reqwest::Response) -> anyhow::Result<Response> {
    let status =
        StatusCode::from_u16(resp.status().as_u16()).unwrap_or(StatusCode::BAD_GATEWAY);
    let headers = resp.headers().clone();
    let bytes = resp.bytes().await?;

    let mut out = Response::builder().status(status);
    let out_headers = out.headers_mut().expect("response builder headers");
    for (name, value) in headers.iter() {
        let key = name.as_str();
        if HOP_BY_HOP.iter().any(|h| h.eq_ignore_ascii_case(key)) {
            continue;
        }
        if let (Ok(n), Ok(v)) = (
            HeaderName::from_bytes(name.as_ref()),
            HeaderValue::from_bytes(value.as_bytes()),
        ) {
            out_headers.insert(n, v);
        }
    }

    Ok(out.body(Body::from(bytes)).unwrap_or_else(|_| {
        (StatusCode::INTERNAL_SERVER_ERROR, "failed to build response").into_response()
    }))
}

/// Build path+query for upstream from Axum URI remainder.
pub fn join_path_query(rest: Option<&str>, uri: &Uri) -> String {
    let path = match rest {
        None | Some("") => "/".to_string(),
        Some(p) if p.starts_with('/') => p.to_string(),
        Some(p) => format!("/{p}"),
    };
    match uri.query() {
        Some(q) => format!("{path}?{q}"),
        None => path,
    }
}
