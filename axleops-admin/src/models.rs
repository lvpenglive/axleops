use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentInfo {
    pub id: String,
    /// Display name, e.g. prod-node-01
    pub name: String,
    /// Base URL of the agent, e.g. http://10.0.0.5:9100 or http://proxy:9200/a/node-01
    pub base_url: String,
    /// Token used when calling the agent API (Agent token or Proxy token)
    pub token: String,
    #[serde(default)]
    pub tags: Vec<String>,
    /// Optional link to a registered axleops-proxy
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub proxy_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RegisterAgentRequest {
    pub name: String,
    pub base_url: String,
    pub token: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub proxy_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpdateAgentRequest {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub base_url: Option<String>,
    #[serde(default)]
    pub token: Option<String>,
    #[serde(default)]
    pub tags: Option<Vec<String>>,
    #[serde(default)]
    pub proxy_id: Option<Option<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyInfo {
    pub id: String,
    pub name: String,
    /// Proxy root, e.g. http://10.0.0.2:9200
    pub base_url: String,
    /// Token for Admin → Proxy
    pub token: String,
    #[serde(default)]
    pub notes: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RegisterProxyRequest {
    pub name: String,
    pub base_url: String,
    pub token: String,
    #[serde(default)]
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpdateProxyRequest {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub base_url: Option<String>,
    #[serde(default)]
    pub token: Option<String>,
    #[serde(default)]
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ImportUpstreamRequest {
    /// Upstream id from proxy config (`[[agents]].id`)
    pub upstream_id: String,
    /// Override agent display name; default upstream.name
    #[serde(default)]
    pub agent_name: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct ProxyHealthView {
    pub proxy_id: String,
    pub proxy_name: String,
    pub reachable: bool,
    pub detail: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpstreamView {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub base_url: String,
    pub path_prefix: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateUpstreamRequest {
    pub id: String,
    #[serde(default)]
    pub name: Option<String>,
    pub base_url: String,
    pub token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateUpstreamRequest {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub base_url: Option<String>,
    #[serde(default)]
    pub token: Option<String>,
}

/// Forward body for starting a service on an agent (jar / script / command).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StartServiceRequest {
    pub name: String,
    #[serde(default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub jar_path: Option<String>,
    #[serde(default)]
    pub script_path: Option<String>,
    #[serde(default)]
    pub command: Option<String>,
    #[serde(default)]
    pub interpreter: Option<String>,
    #[serde(default)]
    pub work_dir: Option<String>,
    #[serde(default)]
    pub jvm_args: Vec<String>,
    #[serde(default)]
    pub app_args: Vec<String>,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub env: HashMap<String, String>,
    #[serde(default)]
    pub health_url: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ApiResponse<T: Serialize> {
    pub ok: bool,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
}

impl<T: Serialize> ApiResponse<T> {
    pub fn ok(message: impl Into<String>, data: T) -> Self {
        Self {
            ok: true,
            message: message.into(),
            data: Some(data),
        }
    }

    pub fn err(message: impl Into<String>) -> ApiResponse<()> {
        ApiResponse {
            ok: false,
            message: message.into(),
            data: None,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct HealthInfo {
    pub service: &'static str,
    pub version: &'static str,
}

#[derive(Debug, Serialize)]
pub struct AgentHealthView {
    pub agent_id: String,
    pub agent_name: String,
    pub reachable: bool,
    pub detail: serde_json::Value,
}

/// One service row in the cross-agent overview.
#[derive(Debug, Serialize)]
pub struct OverviewServiceRow {
    pub agent_id: String,
    pub agent_name: String,
    pub agent_reachable: bool,
    pub name: String,
    pub state: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pid: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub healthy: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct OverviewResponse {
    pub agents_total: usize,
    pub agents_reachable: usize,
    pub services: Vec<OverviewServiceRow>,
}

#[derive(Debug, Deserialize)]
pub struct RotateTokenRequest {
    /// When true, ask the remote Agent to rotate and persist the returned token.
    /// Ignored for proxies (registry-only).
    #[serde(default)]
    pub sync: bool,
}

#[derive(Debug, Serialize)]
pub struct RotateTokenResponse {
    pub id: String,
    /// New token shown once — copy into Agent/Proxy config if sync was false.
    pub token: String,
    pub synced: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}
