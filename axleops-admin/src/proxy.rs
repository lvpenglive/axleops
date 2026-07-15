use crate::models::AgentInfo;
use reqwest::StatusCode;
use serde_json::Value;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ProxyError {
    #[error("agent request failed: {0}")]
    Http(#[from] reqwest::Error),
    #[error("agent returned {status}: {body}")]
    AgentStatus { status: StatusCode, body: String },
    #[error("{0}")]
    Other(String),
}

pub async fn agent_get(
    client: &reqwest::Client,
    agent: &AgentInfo,
    path: &str,
) -> Result<Value, ProxyError> {
    let url = format!("{}{path}", agent.base_url);
    let resp = client
        .get(&url)
        .header("X-AxleOps-Token", &agent.token)
        .send()
        .await?;
    parse_json(resp).await
}

pub async fn agent_post_json(
    client: &reqwest::Client,
    agent: &AgentInfo,
    path: &str,
    body: &impl serde::Serialize,
) -> Result<Value, ProxyError> {
    let url = format!("{}{path}", agent.base_url);
    let resp = client
        .post(&url)
        .header("X-AxleOps-Token", &agent.token)
        .json(body)
        .send()
        .await?;
    parse_json(resp).await
}

pub async fn agent_post_empty(
    client: &reqwest::Client,
    agent: &AgentInfo,
    path: &str,
) -> Result<Value, ProxyError> {
    let url = format!("{}{path}", agent.base_url);
    let resp = client
        .post(&url)
        .header("X-AxleOps-Token", &agent.token)
        .send()
        .await?;
    parse_json(resp).await
}

pub async fn agent_put_json(
    client: &reqwest::Client,
    agent: &AgentInfo,
    path: &str,
    body: &impl serde::Serialize,
) -> Result<Value, ProxyError> {
    let url = format!("{}{path}", agent.base_url);
    let resp = client
        .put(&url)
        .header("X-AxleOps-Token", &agent.token)
        .json(body)
        .send()
        .await?;
    parse_json(resp).await
}

pub async fn agent_delete(
    client: &reqwest::Client,
    agent: &AgentInfo,
    path: &str,
) -> Result<Value, ProxyError> {
    let url = format!("{}{path}", agent.base_url);
    let resp = client
        .delete(&url)
        .header("X-AxleOps-Token", &agent.token)
        .send()
        .await?;
    parse_json(resp).await
}

pub async fn agent_health(
    client: &reqwest::Client,
    agent: &AgentInfo,
) -> Result<Value, ProxyError> {
    let url = format!("{}/health", agent.base_url);
    let resp = client.get(&url).send().await?;
    parse_json(resp).await
}

async fn parse_json(resp: reqwest::Response) -> Result<Value, ProxyError> {
    let status = resp.status();
    let text = resp.text().await.unwrap_or_default();
    if !status.is_success() {
        return Err(ProxyError::AgentStatus {
            status,
            body: text,
        });
    }
    if text.trim().is_empty() {
        return Ok(Value::Null);
    }
    serde_json::from_str(&text).map_err(|e| ProxyError::Other(e.to_string()))
}
