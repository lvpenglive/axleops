use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// How the service is launched.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ServiceKind {
    /// `java [jvm_args] -jar <jar_path> [app_args]`
    Jar,
    /// Script file via inferred/override interpreter (sh/bat/ps1/py/…).
    Script,
    /// Arbitrary executable: `<command> [args]`
    Command,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceSpec {
    /// Unique service name, e.g. order-service
    pub name: String,
    /// Launch mode. If omitted, inferred from jar_path / script_path / command.
    #[serde(default)]
    pub kind: Option<ServiceKind>,

    /// Absolute or relative path to the JAR (`kind=jar`)
    #[serde(default)]
    pub jar_path: Option<String>,
    /// Absolute or relative path to a script (`kind=script`)
    #[serde(default)]
    pub script_path: Option<String>,
    /// Executable for `kind=command`, e.g. "node", "python", "E:\\bin\\worker.exe"
    #[serde(default)]
    pub command: Option<String>,
    /// Optional interpreter override for scripts, e.g. "bash", "powershell", "python"
    #[serde(default)]
    pub interpreter: Option<String>,

    /// Working directory (optional)
    #[serde(default)]
    pub work_dir: Option<String>,
    /// Extra JVM args, e.g. ["-Xms256m", "-Xmx512m"] (`kind=jar`)
    #[serde(default)]
    pub jvm_args: Vec<String>,
    /// App args after -jar (`kind=jar`)
    #[serde(default)]
    pub app_args: Vec<String>,
    /// Args for script / command kinds
    #[serde(default)]
    pub args: Vec<String>,
    /// Process environment variables
    #[serde(default)]
    pub env: HashMap<String, String>,
    /// Health check URL (optional), e.g. http://127.0.0.1:8080/actuator/health
    #[serde(default)]
    pub health_url: Option<String>,
}

impl ServiceSpec {
    /// Resolve launch kind, preferring explicit `kind`.
    pub fn resolved_kind(&self) -> Result<ServiceKind, String> {
        if let Some(kind) = self.kind {
            return Ok(kind);
        }
        if self.jar_path.as_ref().is_some_and(|s| !s.trim().is_empty()) {
            return Ok(ServiceKind::Jar);
        }
        if self.script_path.as_ref().is_some_and(|s| !s.trim().is_empty()) {
            return Ok(ServiceKind::Script);
        }
        if self.command.as_ref().is_some_and(|s| !s.trim().is_empty()) {
            return Ok(ServiceKind::Command);
        }
        Err(
            "kind is required (or provide jar_path / script_path / command)".into(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolved_kind_prefers_explicit() {
        let spec = ServiceSpec {
            name: "x".into(),
            kind: Some(ServiceKind::Command),
            jar_path: Some("a.jar".into()),
            script_path: None,
            command: Some("echo".into()),
            interpreter: None,
            work_dir: None,
            jvm_args: vec![],
            app_args: vec![],
            args: vec![],
            env: HashMap::new(),
            health_url: None,
        };
        assert_eq!(spec.resolved_kind().unwrap(), ServiceKind::Command);
    }

    #[test]
    fn resolved_kind_infers_jar() {
        let spec = ServiceSpec {
            name: "x".into(),
            kind: None,
            jar_path: Some("a.jar".into()),
            script_path: None,
            command: None,
            interpreter: None,
            work_dir: None,
            jvm_args: vec![],
            app_args: vec![],
            args: vec![],
            env: HashMap::new(),
            health_url: None,
        };
        assert_eq!(spec.resolved_kind().unwrap(), ServiceKind::Jar);
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ServiceState {
    Running,
    /// Process is up but health_url probe failed.
    Unhealthy,
    Stopped,
    Unknown,
}

#[derive(Debug, Clone, Serialize)]
pub struct ServiceStatus {
    pub name: String,
    pub state: ServiceState,
    pub kind: Option<ServiceKind>,
    pub pid: Option<u32>,
    /// Jar path, script path, or command string for display.
    pub target: Option<String>,
    /// Legacy alias of `target` when kind is jar (kept for older clients).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub jar_path: Option<String>,
    pub started_at: Option<DateTime<Utc>>,
    pub message: Option<String>,
    /// Result of health_url probe when configured (running/unhealthy only).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub healthy: Option<bool>,
}

impl ServiceStatus {
    pub fn stopped(name: impl Into<String>, message: Option<String>) -> Self {
        Self {
            name: name.into(),
            state: ServiceState::Stopped,
            kind: None,
            pid: None,
            target: None,
            jar_path: None,
            started_at: None,
            message,
            healthy: None,
        }
    }
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
