use crate::config::Config;
use crate::models::{ServiceKind, ServiceSpec, ServiceState, ServiceStatus};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::Mutex;
use std::time::{Duration, Instant};
use sysinfo::{Pid, ProcessesToUpdate, System};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ProcessError {
    #[error("service already running (pid={0})")]
    AlreadyRunning(u32),
    #[error("service is not running")]
    NotRunning,
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("invalid pid file")]
    InvalidPidFile,
    #[error("{0}")]
    Other(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PidRecord {
    pid: u32,
    #[serde(default)]
    kind: String,
    /// Display / restart hint: jar path, script path, or command.
    #[serde(alias = "jar_path")]
    target: String,
    started_at: DateTime<Utc>,
}

/// Persisted service definition (survives stop; enables restart/edit).
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ServiceMeta {
    name: String,
    #[serde(default)]
    kind: String,
    #[serde(default)]
    target: String,
    /// Full launch spec when available (newer agents).
    #[serde(default)]
    spec: Option<ServiceSpec>,
    /// Whether the service should be running (start/stop intent + watchdog).
    #[serde(default)]
    desired_running: bool,
}

pub struct ProcessManager {
    data_dir: PathBuf,
    /// Serialize starts to avoid duplicate PID races under watchdog / concurrent API.
    start_lock: Mutex<()>,
    /// Live OS handles for processes we spawned in this agent lifetime.
    /// Prefer `Child::kill()` over PID-only taskkill/kill — especially on Windows.
    children: Mutex<HashMap<String, Child>>,
    health_start_grace: Duration,
    health_start_interval: Duration,
    health_cache_ttl: Duration,
    health_kill_on_start_fail: bool,
    /// Cached health_url probe results (name -> snapshot).
    health_cache: Mutex<HashMap<String, HealthCacheEntry>>,
}

#[derive(Clone)]
struct HealthCacheEntry {
    at: Instant,
    healthy: bool,
    message: String,
}

#[derive(Clone, Copy)]
enum ProbeMode {
    /// Hit health_url and refresh cache.
    Live,
    /// Use cache if fresh; otherwise leave health unset (no network).
    CacheOnly,
    /// Never probe.
    Skip,
}

impl ProcessManager {
    pub fn new(config: &Config) -> Self {
        let _ = fs::create_dir_all(config.data_dir.join("services"));
        Self {
            data_dir: config.data_dir.clone(),
            start_lock: Mutex::new(()),
            children: Mutex::new(HashMap::new()),
            health_start_grace: Duration::from_secs(config.health_start_grace_secs.max(1)),
            health_start_interval: Duration::from_secs(config.health_start_interval_secs.max(1)),
            health_cache_ttl: Duration::from_secs(config.health_probe_cache_secs),
            health_kill_on_start_fail: config.health_kill_on_start_fail,
            health_cache: Mutex::new(HashMap::new()),
        }
    }

    fn store_child(&self, name: &str, child: Child) {
        if let Ok(mut map) = self.children.lock() {
            // Drop any previous handle for this name without killing it;
            // caller is responsible for stopping first.
            map.insert(name.to_string(), child);
        }
    }

    fn take_child(&self, name: &str) -> Option<Child> {
        self.children
            .lock()
            .ok()
            .and_then(|mut map| map.remove(name))
    }

    /// Reap exited children so PID files / status stay accurate.
    fn reap_child_if_exited(&self, name: &str) -> bool {
        let Ok(mut map) = self.children.lock() else {
            return false;
        };
        let Some(child) = map.get_mut(name) else {
            return false;
        };
        match child.try_wait() {
            Ok(Some(_)) => {
                map.remove(name);
                true
            }
            Ok(None) => false,
            Err(_) => {
                map.remove(name);
                true
            }
        }
    }

    fn pid_path(&self, name: &str) -> PathBuf {
        self.data_dir.join("pids").join(format!("{name}.pid.json"))
    }

    fn log_path(&self, name: &str) -> PathBuf {
        self.data_dir.join("logs").join(format!("{name}.log"))
    }

    fn service_meta_path(&self, name: &str) -> PathBuf {
        self.data_dir.join("services").join(format!("{name}.json"))
    }

    fn read_meta(&self, name: &str) -> Result<Option<ServiceMeta>, ProcessError> {
        let path = self.service_meta_path(name);
        if !path.exists() {
            return Ok(None);
        }
        let text = fs::read_to_string(path)?;
        let meta: ServiceMeta =
            serde_json::from_str(&text).map_err(|e| ProcessError::Other(e.to_string()))?;
        Ok(Some(meta))
    }

    fn write_meta(&self, meta: &ServiceMeta) -> Result<(), ProcessError> {
        let dir = self.data_dir.join("services");
        fs::create_dir_all(&dir)?;
        let text = serde_json::to_string_pretty(meta)
            .map_err(|e| ProcessError::Other(e.to_string()))?;
        fs::write(self.service_meta_path(&meta.name), text)?;
        Ok(())
    }

    fn meta_from_spec(spec: &ServiceSpec) -> Result<ServiceMeta, ProcessError> {
        let kind = spec.resolved_kind().map_err(ProcessError::Other)?;
        let target = match kind {
            ServiceKind::Jar => spec.jar_path.clone().unwrap_or_default(),
            ServiceKind::Script => spec.script_path.clone().unwrap_or_default(),
            ServiceKind::Command => spec.command.clone().unwrap_or_default(),
        };
        Ok(ServiceMeta {
            name: spec.name.trim().to_string(),
            kind: Self::kind_str(kind).to_string(),
            target,
            spec: Some(spec.clone()),
            desired_running: false,
        })
    }

    /// Reconstruct a minimal spec from legacy meta that only stored kind/target.
    fn spec_from_meta(meta: &ServiceMeta) -> Result<ServiceSpec, ProcessError> {
        if let Some(spec) = &meta.spec {
            return Ok(spec.clone());
        }
        let kind = Self::parse_kind(&meta.kind).ok_or_else(|| {
            ProcessError::Other("service has no saved spec; edit and save first".into())
        })?;
        let mut spec = ServiceSpec {
            name: meta.name.clone(),
            kind: Some(kind),
            jar_path: None,
            script_path: None,
            command: None,
            interpreter: None,
            work_dir: None,
            jvm_args: Vec::new(),
            app_args: Vec::new(),
            args: Vec::new(),
            env: HashMap::new(),
            health_url: None,
        };
        match kind {
            ServiceKind::Jar => spec.jar_path = Some(meta.target.clone()),
            ServiceKind::Script => spec.script_path = Some(meta.target.clone()),
            ServiceKind::Command => spec.command = Some(meta.target.clone()),
        }
        Ok(spec)
    }

    /// Save / update definition without starting.
    pub fn save(&self, spec: &ServiceSpec) -> Result<ServiceStatus, ProcessError> {
        if spec.name.trim().is_empty() {
            return Err(ProcessError::Other("name is required".into()));
        }
        let _ = spec.resolved_kind().map_err(ProcessError::Other)?;
        let mut meta = Self::meta_from_spec(spec)?;
        // Preserve watchdog intent — editing a definition must not clear desired_running.
        if let Ok(Some(prev)) = self.read_meta(&meta.name) {
            meta.desired_running = prev.desired_running;
        }
        self.write_meta(&meta)?;
        Ok(self.status_ex(&meta.name, ProbeMode::CacheOnly))
    }

    pub fn get_spec(&self, name: &str) -> Result<ServiceSpec, ProcessError> {
        let meta = self
            .read_meta(name)?
            .ok_or_else(|| ProcessError::Other(format!("service not found: {name}")))?;
        Self::spec_from_meta(&meta)
    }

    /// Start using the saved definition.
    pub fn start_saved(&self, name: &str) -> Result<ServiceStatus, ProcessError> {
        let spec = self.get_spec(name)?;
        self.start(&spec)
    }

    fn read_record(&self, name: &str) -> Result<Option<PidRecord>, ProcessError> {
        let path = self.pid_path(name);
        if !path.exists() {
            return Ok(None);
        }
        let text = fs::read_to_string(path)?;
        let record: PidRecord =
            serde_json::from_str(&text).map_err(|_| ProcessError::InvalidPidFile)?;
        Ok(Some(record))
    }

    fn write_record(&self, name: &str, record: &PidRecord) -> Result<(), ProcessError> {
        let path = self.pid_path(name);
        let text = serde_json::to_string_pretty(record)
            .map_err(|e| ProcessError::Other(e.to_string()))?;
        fs::write(path, text)?;
        Ok(())
    }

    fn clear_record(&self, name: &str) -> Result<(), ProcessError> {
        let path = self.pid_path(name);
        if path.exists() {
            fs::remove_file(path)?;
        }
        Ok(())
    }

    fn is_pid_alive(pid: u32) -> bool {
        let mut sys = System::new();
        sys.refresh_processes(ProcessesToUpdate::Some(&[Pid::from_u32(pid)]), true);
        sys.process(Pid::from_u32(pid)).is_some()
    }

    /// All descendants of `root` (not including root), via current process table.
    fn collect_descendant_pids(root: u32) -> Vec<u32> {
        let mut sys = System::new();
        sys.refresh_processes(ProcessesToUpdate::All, true);
        let mut by_parent: HashMap<u32, Vec<u32>> = HashMap::new();
        for (pid, proc) in sys.processes() {
            if let Some(parent) = proc.parent() {
                by_parent
                    .entry(parent.as_u32())
                    .or_default()
                    .push(pid.as_u32());
            }
        }
        let mut out = Vec::new();
        let mut stack = vec![root];
        let mut seen = std::collections::HashSet::new();
        seen.insert(root);
        while let Some(p) = stack.pop() {
            let Some(children) = by_parent.get(&p) else {
                continue;
            };
            for &c in children {
                if seen.insert(c) {
                    out.push(c);
                    stack.push(c);
                }
            }
        }
        out
    }

    fn collect_tree_pids(root: u32) -> Vec<u32> {
        let mut pids = Self::collect_descendant_pids(root);
        pids.push(root);
        pids.sort_unstable();
        pids.dedup();
        pids
    }

    fn any_pid_alive(pids: &[u32]) -> bool {
        pids.iter().copied().any(Self::is_pid_alive)
    }

    fn refresh_tree_pids(root: u32, known: &[u32]) -> Vec<u32> {
        let mut set: std::collections::BTreeSet<u32> =
            known.iter().copied().filter(|p| Self::is_pid_alive(*p)).collect();
        if Self::is_pid_alive(root) {
            for p in Self::collect_tree_pids(root) {
                set.insert(p);
            }
        }
        set.into_iter().collect()
    }

    fn parse_kind(s: &str) -> Option<ServiceKind> {
        match s {
            "jar" => Some(ServiceKind::Jar),
            "script" => Some(ServiceKind::Script),
            "command" => Some(ServiceKind::Command),
            _ => None,
        }
    }

    fn kind_str(kind: ServiceKind) -> &'static str {
        match kind {
            ServiceKind::Jar => "jar",
            ServiceKind::Script => "script",
            ServiceKind::Command => "command",
        }
    }

    fn status_running(name: &str, record: &PidRecord) -> ServiceStatus {
        let kind = Self::parse_kind(&record.kind);
        let jar_path = if kind == Some(ServiceKind::Jar) {
            Some(record.target.clone())
        } else {
            None
        };
        ServiceStatus {
            name: name.to_string(),
            state: ServiceState::Running,
            kind,
            pid: Some(record.pid),
            target: Some(record.target.clone()),
            jar_path,
            started_at: Some(record.started_at),
            message: None,
            healthy: None,
        }
    }

    fn status_stopped(
        name: &str,
        kind: Option<ServiceKind>,
        target: Option<String>,
        message: Option<String>,
    ) -> ServiceStatus {
        let jar_path = if kind == Some(ServiceKind::Jar) {
            target.clone()
        } else {
            None
        };
        ServiceStatus {
            name: name.to_string(),
            state: ServiceState::Stopped,
            kind,
            pid: None,
            target,
            jar_path,
            started_at: None,
            message,
            healthy: None,
        }
    }

    fn apply_health(mut status: ServiceStatus, health_url: Option<&str>) -> ServiceStatus {
        let Some(url) = health_url.map(str::trim).filter(|u| !u.is_empty()) else {
            return status;
        };
        match probe_health_url(url) {
            Ok(()) => {
                status.healthy = Some(true);
                if status.message.is_none() {
                    status.message = Some("healthy".into());
                }
            }
            Err(err) => {
                status.state = ServiceState::Unhealthy;
                status.healthy = Some(false);
                status.message = Some(format!("unhealthy: {err}"));
            }
        }
        status
    }

    fn remember_health(&self, name: &str, status: &ServiceStatus) {
        let Some(healthy) = status.healthy else {
            return;
        };
        if let Ok(mut cache) = self.health_cache.lock() {
            cache.insert(
                name.to_string(),
                HealthCacheEntry {
                    at: Instant::now(),
                    healthy,
                    message: status
                        .message
                        .clone()
                        .unwrap_or_else(|| {
                            if healthy {
                                "healthy".into()
                            } else {
                                "unhealthy".into()
                            }
                        }),
                },
            );
        }
    }

    fn apply_cached_health(&self, name: &str, mut status: ServiceStatus) -> ServiceStatus {
        let Ok(cache) = self.health_cache.lock() else {
            return status;
        };
        let Some(entry) = cache.get(name) else {
            return status;
        };
        if self.health_cache_ttl.is_zero() || entry.at.elapsed() > self.health_cache_ttl {
            return status;
        }
        status.healthy = Some(entry.healthy);
        status.message = Some(entry.message.clone());
        if entry.healthy {
            if matches!(status.state, ServiceState::Unhealthy) {
                status.state = ServiceState::Running;
            }
        } else if matches!(status.state, ServiceState::Running) {
            status.state = ServiceState::Unhealthy;
        }
        status
    }

    fn with_health(
        &self,
        name: &str,
        status: ServiceStatus,
        health_url: Option<&str>,
        mode: ProbeMode,
    ) -> ServiceStatus {
        let url = health_url.map(str::trim).filter(|u| !u.is_empty());
        if url.is_none() || !matches!(status.state, ServiceState::Running | ServiceState::Unhealthy) {
            return status;
        }
        match mode {
            ProbeMode::Skip => status,
            ProbeMode::CacheOnly => self.apply_cached_health(name, status),
            ProbeMode::Live => {
                let probed = Self::apply_health(status, url);
                self.remember_health(name, &probed);
                probed
            }
        }
    }

    pub fn status(&self, name: &str) -> ServiceStatus {
        self.status_ex(name, ProbeMode::Live)
    }

    fn status_ex(&self, name: &str, mode: ProbeMode) -> ServiceStatus {
        let meta = self.read_meta(name).ok().flatten();
        let health_url = meta
            .as_ref()
            .and_then(|m| m.spec.as_ref())
            .and_then(|s| s.health_url.as_deref());

        // Reap exited Child handles so we don't leave zombies / stale tracking.
        let _ = self.reap_child_if_exited(name);

        match self.read_record(name) {
            Ok(Some(record)) if Self::is_pid_alive(record.pid) => {
                if meta.is_none() {
                    let _ = self.write_meta(&ServiceMeta {
                        name: name.to_string(),
                        kind: record.kind.clone(),
                        target: record.target.clone(),
                        spec: None,
                        desired_running: false,
                    });
                }
                let status = Self::status_running(name, &record);
                self.with_health(name, status, health_url, mode)
            }
            Ok(Some(record)) => {
                let _ = self.take_child(name);
                let _ = self.clear_record(name);
                if let Ok(mut cache) = self.health_cache.lock() {
                    cache.remove(name);
                }
                let kind = meta
                    .as_ref()
                    .and_then(|m| Self::parse_kind(&m.kind))
                    .or_else(|| Self::parse_kind(&record.kind));
                let target = meta
                    .as_ref()
                    .map(|m| m.target.clone())
                    .filter(|t| !t.is_empty())
                    .or(Some(record.target.clone()));
                if meta.is_none() {
                    let _ = self.write_meta(&ServiceMeta {
                        name: name.to_string(),
                        kind: record.kind,
                        target: record.target,
                        spec: None,
                        desired_running: false,
                    });
                }
                Self::status_stopped(name, kind, target, Some("stale pid cleared".into()))
            }
            Ok(None) => {
                let _ = self.take_child(name);
                let kind = meta.as_ref().and_then(|m| Self::parse_kind(&m.kind));
                let target = meta
                    .as_ref()
                    .map(|m| m.target.clone())
                    .filter(|t| !t.is_empty());
                Self::status_stopped(name, kind, target, None)
            }
            Err(e) => ServiceStatus {
                name: name.to_string(),
                state: ServiceState::Unknown,
                kind: meta.as_ref().and_then(|m| Self::parse_kind(&m.kind)),
                pid: None,
                target: meta.as_ref().map(|m| m.target.clone()),
                jar_path: None,
                started_at: None,
                message: Some(e.to_string()),
                healthy: None,
            },
        }
    }

    /// Restart = stop (if needed) then start from saved spec.
    pub fn restart(&self, name: &str) -> Result<ServiceStatus, ProcessError> {
        let current = self.status_ex(name, ProbeMode::Skip);
        if current.pid.is_some() {
            self.stop_runtime(name, false)?;
        }
        self.start_saved(name)
    }

    /// Remove definition (+ stop if running). Deletes meta and pid; keeps log file.
    pub fn remove(&self, name: &str) -> Result<ServiceStatus, ProcessError> {
        let current = self.status_ex(name, ProbeMode::Skip);
        if current.pid.is_some() {
            let _ = self.stop(name);
        } else {
            let _ = self.clear_record(name);
        }
        let meta_path = self.service_meta_path(name);
        if meta_path.exists() {
            fs::remove_file(&meta_path)?;
        }
        Ok(ServiceStatus::stopped(name, Some("removed".into())))
    }

    fn collect_service_names(&self) -> Result<Vec<String>, ProcessError> {
        let mut names = std::collections::BTreeSet::new();
        for sub in ["services", "pids"] {
            let dir = self.data_dir.join(sub);
            if !dir.exists() {
                continue;
            }
            for entry in fs::read_dir(dir)? {
                let entry = entry?;
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) != Some("json") {
                    continue;
                }
                let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
                    continue;
                };
                let name = if sub == "pids" {
                    stem.trim_end_matches(".pid").to_string()
                } else {
                    stem.to_string()
                };
                if !name.is_empty() {
                    names.insert(name);
                }
            }
        }
        Ok(names.into_iter().collect())
    }

    pub fn list_statuses(&self) -> Result<Vec<ServiceStatus>, ProcessError> {
        let names = self.collect_service_names()?;
        // Avoid N×health_url on every Admin refresh; use cache or skip.
        Ok(names
            .iter()
            .map(|n| self.status_ex(n, ProbeMode::CacheOnly))
            .collect())
    }

    pub fn start(&self, spec: &ServiceSpec) -> Result<ServiceStatus, ProcessError> {
        let _guard = self
            .start_lock
            .lock()
            .map_err(|_| ProcessError::Other("start lock poisoned".into()))?;

        let current = self.status_ex(&spec.name, ProbeMode::Skip);
        if current.pid.is_some()
            && matches!(
                current.state,
                ServiceState::Running | ServiceState::Unhealthy
            )
        {
            return Err(ProcessError::AlreadyRunning(current.pid.unwrap()));
        }

        let kind = spec
            .resolved_kind()
            .map_err(ProcessError::Other)?;

        let (mut cmd, target, work_dir) = match kind {
            ServiceKind::Jar => self.build_jar_command(spec)?,
            ServiceKind::Script => self.build_script_command(spec)?,
            ServiceKind::Command => self.build_exec_command(spec)?,
        };

        for (k, v) in &spec.env {
            cmd.env(k, v);
        }

        let log_file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(self.log_path(&spec.name))?;

        cmd.current_dir(&work_dir)
            .stdin(Stdio::null())
            .stdout(Stdio::from(log_file.try_clone()?))
            .stderr(Stdio::from(log_file));

        // Detach so the service is not bound to the agent console/session.
        // Killing the service must not require killing the agent (and vice versa).
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            // setsid(): new session + process group, leave agent's controlling TTY.
            // After this, PGID == SID == child pid — stop can signal the whole tree with -pid.
            unsafe {
                cmd.pre_exec(|| {
                    if libc::setsid() == -1 {
                        return Err(std::io::Error::last_os_error());
                    }
                    Ok(())
                });
            }
        }
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            // CREATE_NEW_PROCESS_GROUP | CREATE_NO_WINDOW
            const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
            const CREATE_NO_WINDOW: u32 = 0x0800_0000;
            cmd.creation_flags(CREATE_NEW_PROCESS_GROUP | CREATE_NO_WINDOW);
        }

        let mut child: Child = cmd.spawn()?;
        let pid = child.id();
        let started_at = Utc::now();

        let mut meta = Self::meta_from_spec(spec)?;
        meta.desired_running = true;
        if let Err(e) = self.write_meta(&meta) {
            let _ = child.kill();
            let _ = child.wait();
            return Err(e);
        }

        if let Err(e) = self.write_record(
            &spec.name,
            &PidRecord {
                pid,
                kind: Self::kind_str(kind).to_string(),
                target: target.clone(),
                started_at,
            },
        ) {
            let _ = child.kill();
            let _ = child.wait();
            if let Ok(Some(mut meta)) = self.read_meta(&spec.name) {
                meta.desired_running = false;
                let _ = self.write_meta(&meta);
            }
            return Err(e);
        }

        // Keep the OS handle so stop can TerminateProcess / SIGKILL without relying on PID alone.
        self.store_child(&spec.name, child);

        // Catch immediate crash (bad jar / missing main / fatal config).
        std::thread::sleep(std::time::Duration::from_millis(800));
        if self.reap_child_if_exited(&spec.name) || !Self::is_pid_alive(pid) {
            let _ = self.take_child(&spec.name);
            let _ = self.clear_record(&spec.name);
            if let Ok(Some(mut meta)) = self.read_meta(&spec.name) {
                meta.desired_running = false;
                let _ = self.write_meta(&meta);
            }
            return Err(ProcessError::Other(format!(
                "process exited immediately after start (pid={pid}); check service logs"
            )));
        }

        let jar_path = if kind == ServiceKind::Jar {
            Some(target.clone())
        } else {
            None
        };

        let mut status = ServiceStatus {
            name: spec.name.clone(),
            state: ServiceState::Running,
            kind: Some(kind),
            pid: Some(pid),
            target: Some(target),
            jar_path,
            started_at: Some(started_at),
            message: Some("started".into()),
            healthy: None,
        };

        if let Some(url) = spec.health_url.as_deref().map(str::trim).filter(|u| !u.is_empty()) {
            // Retry within grace window — slow Spring JARs often need >2s.
            let deadline = Instant::now() + self.health_start_grace;
            loop {
                if self.reap_child_if_exited(&spec.name) || !Self::is_pid_alive(pid) {
                    let _ = self.take_child(&spec.name);
                    let _ = self.clear_record(&spec.name);
                    if let Ok(Some(mut meta)) = self.read_meta(&spec.name) {
                        meta.desired_running = false;
                        let _ = self.write_meta(&meta);
                    }
                    return Err(ProcessError::Other(format!(
                        "process exited during health wait (pid={pid}); check service logs"
                    )));
                }

                match probe_health_url(url) {
                    Ok(()) => {
                        status.healthy = Some(true);
                        status.state = ServiceState::Running;
                        status.message = Some("healthy".into());
                        self.remember_health(&spec.name, &status);
                        break;
                    }
                    Err(err) => {
                        if Instant::now() >= deadline {
                            status.state = ServiceState::Unhealthy;
                            status.healthy = Some(false);
                            status.message = Some(format!("unhealthy: {err}"));
                            self.remember_health(&spec.name, &status);
                            if self.health_kill_on_start_fail {
                                let msg = status
                                    .message
                                    .clone()
                                    .unwrap_or_else(|| "health check failed".into());
                                if let Err(e) = self.stop_runtime(&spec.name, true) {
                                    tracing::warn!(
                                        service = %spec.name,
                                        error = %e,
                                        "cleanup after failed health probe also failed"
                                    );
                                    return Err(ProcessError::Other(format!(
                                        "start failed ({msg}); process cleanup failed: {e}"
                                    )));
                                }
                                return Err(ProcessError::Other(format!(
                                    "start failed ({msg}); process stopped"
                                )));
                            }
                            // Keep process; Admin can stop explicitly. Avoid false kills on slow boot.
                            break;
                        }
                        std::thread::sleep(self.health_start_interval);
                    }
                }
            }
        }

        Ok(status)
    }

    fn build_jar_command(
        &self,
        spec: &ServiceSpec,
    ) -> Result<(Command, String, PathBuf), ProcessError> {
        let jar_path = spec
            .jar_path
            .as_ref()
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .ok_or_else(|| ProcessError::Other("jar_path is required for kind=jar".into()))?
            .to_string();

        let jar = Path::new(&jar_path);
        if !jar.exists() {
            return Err(ProcessError::Other(format!("jar not found: {jar_path}")));
        }

        let work_dir = resolve_work_dir(spec.work_dir.as_deref(), jar.parent())?;

        let mut cmd = Command::new("java");
        cmd.args(&spec.jvm_args)
            .arg("-jar")
            .arg(&jar_path)
            .args(&spec.app_args)
            .args(&spec.args);

        Ok((cmd, jar_path, work_dir))
    }

    fn build_script_command(
        &self,
        spec: &ServiceSpec,
    ) -> Result<(Command, String, PathBuf), ProcessError> {
        let script_path = spec
            .script_path
            .as_ref()
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .ok_or_else(|| ProcessError::Other("script_path is required for kind=script".into()))?
            .to_string();

        let script = Path::new(&script_path);
        if !script.exists() {
            return Err(ProcessError::Other(format!(
                "script not found: {script_path}"
            )));
        }

        let work_dir = resolve_work_dir(spec.work_dir.as_deref(), script.parent())?;
        let launch = resolve_script_launcher(script, spec.interpreter.as_deref())?;

        let cmd = match launch {
            ScriptLaunch::Direct => {
                let mut c = Command::new(&script_path);
                c.args(&spec.args);
                c
            }
            ScriptLaunch::Via {
                program,
                prefix_args,
            } => {
                let mut c = Command::new(&program);
                c.args(&prefix_args).arg(&script_path).args(&spec.args);
                c
            }
        };

        Ok((cmd, script_path, work_dir))
    }

    fn build_exec_command(
        &self,
        spec: &ServiceSpec,
    ) -> Result<(Command, String, PathBuf), ProcessError> {
        let command = spec
            .command
            .as_ref()
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .ok_or_else(|| ProcessError::Other("command is required for kind=command".into()))?
            .to_string();

        let work_dir = resolve_work_dir(spec.work_dir.as_deref(), Path::new(&command).parent())?;

        let mut cmd = Command::new(&command);
        cmd.args(&spec.args);

        Ok((cmd, command, work_dir))
    }

    pub fn stop(&self, name: &str) -> Result<ServiceStatus, ProcessError> {
        self.stop_runtime(name, true)
    }

    fn stop_runtime(&self, name: &str, clear_desired: bool) -> Result<ServiceStatus, ProcessError> {
        let record = self
            .read_record(name)?
            .ok_or(ProcessError::NotRunning)?;

        let mut child = self.take_child(name);

        // Prefer owned handle for liveness; fall back to PID scan.
        let alive = match child.as_mut().and_then(|c| c.try_wait().ok()) {
            Some(Some(_)) => false, // already exited
            Some(None) => true,
            None => Self::is_pid_alive(record.pid) || !Self::collect_descendant_pids(record.pid).is_empty(),
        };

        if !alive {
            let _ = self.clear_record(name);
            if let Ok(mut cache) = self.health_cache.lock() {
                cache.remove(name);
            }
            if let Some(mut c) = child {
                let _ = c.wait();
            }
            return Err(ProcessError::NotRunning);
        }

        // Snapshot the tree before signalling — children may outlive the root pid.
        let mut tree = Self::collect_tree_pids(record.pid);

        // Soft stop: process group + every known descendant.
        let _ = terminate_tree(record.pid, &tree);

        for _ in 0..40 {
            tree = Self::refresh_tree_pids(record.pid, &tree);
            let child_gone = match child.as_mut().and_then(|c| c.try_wait().ok()) {
                Some(Some(_)) => true,
                Some(None) => false,
                None => true,
            };
            if child_gone && !Self::any_pid_alive(&tree) {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }

        tree = Self::refresh_tree_pids(record.pid, &tree);
        let still_alive = match child.as_mut().and_then(|c| c.try_wait().ok()) {
            Some(Some(_)) => Self::any_pid_alive(&tree),
            Some(None) => true,
            None => Self::any_pid_alive(&tree),
        };

        if still_alive {
            // Owned handle → TerminateProcess (Windows) / SIGKILL (Unix).
            if let Some(ref mut c) = child {
                let _ = c.kill();
            }
            // Force-kill whole tree (group + each descendant + root).
            let _ = force_kill_tree(record.pid, &tree);

            for _ in 0..30 {
                tree = Self::refresh_tree_pids(record.pid, &tree);
                let child_gone = match child.as_mut().and_then(|c| c.try_wait().ok()) {
                    Some(Some(_)) => true,
                    Some(None) => false,
                    None => true,
                };
                if child_gone && !Self::any_pid_alive(&tree) {
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(100));
            }

            // Second force pass for stubborn grandchildren spawned during the first kill.
            tree = Self::refresh_tree_pids(record.pid, &tree);
            if Self::any_pid_alive(&tree)
                || matches!(child.as_mut().and_then(|c| c.try_wait().ok()), Some(None))
            {
                if let Some(ref mut c) = child {
                    let _ = c.kill();
                }
                let _ = force_kill_tree(record.pid, &tree);
                for _ in 0..20 {
                    tree = Self::refresh_tree_pids(record.pid, &tree);
                    let child_gone = match child.as_mut().and_then(|c| c.try_wait().ok()) {
                        Some(Some(_)) => true,
                        Some(None) => false,
                        None => true,
                    };
                    if child_gone && !Self::any_pid_alive(&tree) {
                        break;
                    }
                    std::thread::sleep(std::time::Duration::from_millis(100));
                }
            }

            tree = Self::refresh_tree_pids(record.pid, &tree);
            let still = match child.as_mut().and_then(|c| c.try_wait().ok()) {
                Some(Some(_)) => Self::any_pid_alive(&tree),
                Some(None) => true,
                None => Self::any_pid_alive(&tree),
            };
            if still {
                if let Some(c) = child {
                    self.store_child(name, c);
                }
                let leftover: Vec<u32> = tree.into_iter().filter(|p| Self::is_pid_alive(*p)).collect();
                return Err(ProcessError::Other(format!(
                    "process tree still alive after force kill (root={}, leftover={leftover:?})",
                    record.pid
                )));
            }
        }

        if let Some(mut c) = child {
            let _ = c.wait();
        }

        // Keep existing definition (incl. full spec); only clear runtime pid.
        if self.read_meta(name)?.is_none() {
            let _ = self.write_meta(&ServiceMeta {
                name: name.to_string(),
                kind: record.kind.clone(),
                target: record.target.clone(),
                spec: None,
                desired_running: false,
            });
        }
        self.clear_record(name)?;
        if let Ok(mut cache) = self.health_cache.lock() {
            cache.remove(name);
        }

        if clear_desired {
            if let Ok(Some(mut meta)) = self.read_meta(name) {
                meta.desired_running = false;
                let _ = self.write_meta(&meta);
            }
        }

        let kind = Self::parse_kind(&record.kind);
        let jar_path = if kind == Some(ServiceKind::Jar) {
            Some(record.target.clone())
        } else {
            None
        };

        Ok(ServiceStatus {
            name: name.to_string(),
            state: ServiceState::Stopped,
            kind,
            pid: None,
            target: Some(record.target),
            jar_path,
            started_at: None,
            message: Some("stopped".into()),
            healthy: None,
        })
    }

    /// On Agent boot: start services previously marked desired_running.
    pub fn recover_desired(&self) {
        let Ok(names) = self.collect_service_names() else {
            return;
        };
        for name in names {
            let Ok(Some(meta)) = self.read_meta(&name) else {
                continue;
            };
            if !meta.desired_running {
                continue;
            }
            match self.start_saved(&name) {
                Ok(st) => tracing::info!(service = %name, pid = ?st.pid, "recovered desired service"),
                Err(ProcessError::AlreadyRunning(pid)) => {
                    tracing::info!(service = %name, pid, "desired service already running")
                }
                Err(e) => tracing::warn!(service = %name, error = %e, "failed to recover desired service"),
            }
        }
    }

    /// Periodic watchdog: restart desired services that exited or became unhealthy.
    pub fn watchdog_tick(&self) {
        let Ok(names) = self.collect_service_names() else {
            return;
        };
        for name in names {
            let Ok(Some(meta)) = self.read_meta(&name) else {
                continue;
            };
            if !meta.desired_running {
                continue;
            }
            let st = self.status_ex(&name, ProbeMode::Skip);
            let need_restart = match st.state {
                ServiceState::Running | ServiceState::Unhealthy => false,
                ServiceState::Stopped | ServiceState::Unknown => true,
            };
            if !need_restart {
                continue;
            }
            if st.pid.is_some() {
                if let Err(e) = self.stop_runtime(&name, false) {
                    tracing::warn!(service = %name, error = %e, "watchdog stop before restart failed");
                }
            }
            match self.start_saved(&name) {
                Ok(st) => tracing::warn!(
                    service = %name,
                    pid = ?st.pid,
                    "watchdog restarted desired service"
                ),
                Err(ProcessError::AlreadyRunning(_)) => {}
                Err(e) => tracing::warn!(service = %name, error = %e, "watchdog restart failed"),
            }
        }
    }

    pub fn tail_log(&self, name: &str, max_bytes: u64) -> Result<String, ProcessError> {
        let path = self.log_path(name);
        if !path.exists() {
            return Ok(String::new());
        }
        let meta = fs::metadata(&path)?;
        let file = fs::File::open(&path)?;
        use std::io::{Read, Seek, SeekFrom};

        let mut file = file;
        let start = meta.len().saturating_sub(max_bytes);
        file.seek(SeekFrom::Start(start))?;
        let mut buf = Vec::new();
        file.read_to_end(&mut buf)?;
        Ok(decode_log_bytes(&buf))
    }
}

/// Decode log bytes: UTF-8 first, then Windows GBK, else lossy UTF-8.
fn decode_log_bytes(buf: &[u8]) -> String {
    if let Ok(s) = std::str::from_utf8(buf) {
        return s.to_string();
    }
    let (cow, _, had_errors) = encoding_rs::GBK.decode(buf);
    if !had_errors {
        return cow.into_owned();
    }
    String::from_utf8_lossy(buf).into_owned()
}

fn probe_health_url(url: &str) -> Result<(), String> {
    let resp = ureq::get(url)
        .timeout(std::time::Duration::from_secs(3))
        .call()
        .map_err(|e| e.to_string())?;
    let code = resp.status();
    if (200..400).contains(&code) {
        Ok(())
    } else {
        Err(format!("HTTP {code}"))
    }
}

fn resolve_work_dir(
    explicit: Option<&str>,
    fallback_parent: Option<&Path>,
) -> Result<PathBuf, ProcessError> {
    let dir = if let Some(dir) = explicit.map(str::trim).filter(|s| !s.is_empty()) {
        PathBuf::from(dir)
    } else {
        fallback_parent
            .filter(|p| !p.as_os_str().is_empty())
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."))
    };

    if !dir.exists() {
        return Err(ProcessError::Other(format!(
            "work_dir not found: {}",
            dir.display()
        )));
    }
    if !dir.is_dir() {
        return Err(ProcessError::Other(format!(
            "work_dir is not a directory: {}",
            dir.display()
        )));
    }
    Ok(dir)
}

enum ScriptLaunch {
    /// Run the script path itself (Unix +x / Windows .exe).
    Direct,
    /// `program [prefix_args...] <script> [args...]`
    Via {
        program: String,
        prefix_args: Vec<String>,
    },
}

/// Pick interpreter + prepend args for a script path.
fn resolve_script_launcher(
    script: &Path,
    interpreter: Option<&str>,
) -> Result<ScriptLaunch, ProcessError> {
    if let Some(interp) = interpreter.map(str::trim).filter(|s| !s.is_empty()) {
        return Ok(launcher_for_interpreter(interp));
    }

    let ext = script
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();

    match ext.as_str() {
        "sh" | "bash" => Ok(launcher_for_interpreter("bash")),
        "zsh" => Ok(launcher_for_interpreter("zsh")),
        "ps1" => Ok(launcher_for_interpreter("powershell")),
        "bat" | "cmd" => Ok(launcher_for_interpreter("cmd")),
        "py" | "pyw" => Ok(launcher_for_interpreter("python")),
        "js" | "mjs" | "cjs" => Ok(launcher_for_interpreter("node")),
        "rb" => Ok(launcher_for_interpreter("ruby")),
        "pl" => Ok(launcher_for_interpreter("perl")),
        // Executable / no extension: run path directly (needs +x on Unix).
        "" | "exe" | "com" => Ok(ScriptLaunch::Direct),
        other => Err(ProcessError::Other(format!(
            "unsupported script extension '.{other}'; set interpreter explicitly"
        ))),
    }
}

fn launcher_for_interpreter(interp: &str) -> ScriptLaunch {
    let lower = interp.to_ascii_lowercase();
    match lower.as_str() {
        "powershell" | "pwsh" => ScriptLaunch::Via {
            program: if lower == "pwsh" {
                "pwsh".into()
            } else {
                "powershell".into()
            },
            prefix_args: vec![
                "-NoProfile".into(),
                "-ExecutionPolicy".into(),
                "Bypass".into(),
                "-File".into(),
            ],
        },
        "cmd" | "cmd.exe" => ScriptLaunch::Via {
            program: "cmd".into(),
            prefix_args: vec!["/C".into()],
        },
        _ => ScriptLaunch::Via {
            program: interp.to_string(),
            prefix_args: Vec::new(),
        },
    }
}

fn run_taskkill(args: &[&str], pid: u32) -> Result<(), ProcessError> {
    let output = Command::new("taskkill").args(args).output()?;
    if output.status.success() {
        return Ok(());
    }
    let stderr = decode_log_bytes(&output.stderr);
    let stdout = decode_log_bytes(&output.stdout);
    let combined = format!("{stdout}{stderr}").to_ascii_lowercase();
    // Process already gone — treat as success.
    if combined.contains("not found")
        || combined.contains("not running")
        || combined.contains("找不到")
        || combined.contains("没有运行")
    {
        return Ok(());
    }
    let detail = {
        let s = stderr.trim();
        if s.is_empty() {
            stdout.trim()
        } else {
            s
        }
    };
    Err(ProcessError::Other(format!(
        "taskkill {} failed for pid {pid}: {detail}",
        args.join(" ")
    )))
}

fn terminate_tree(root: u32, tree: &[u32]) -> Result<(), ProcessError> {
    #[cfg(windows)]
    {
        let root_s = root.to_string();
        // Soft terminate process tree; failure is OK — caller may force-kill.
        let _ = run_taskkill(&["/PID", &root_s, "/T"], root);
        for &pid in tree.iter().rev() {
            if pid == root {
                continue;
            }
            let pid_s = pid.to_string();
            let _ = run_taskkill(&["/PID", &pid_s], pid);
        }
        Ok(())
    }
    #[cfg(unix)]
    {
        // Soft signal process group first (setsid → PGID == root).
        unix_kill_group(root, libc::SIGTERM);
        // Then every known descendant (covers processes that left the group).
        for &pid in tree.iter().rev() {
            unix_kill_one(pid, libc::SIGTERM);
        }
        Ok(())
    }
}

fn force_kill_tree(root: u32, tree: &[u32]) -> Result<(), ProcessError> {
    #[cfg(windows)]
    {
        let root_s = root.to_string();
        let mut last_err = None;
        // /T kills the Windows process tree rooted at root.
        if let Err(e) = run_taskkill(&["/F", "/T", "/PID", &root_s], root) {
            last_err = Some(e);
        }
        for &pid in tree.iter().rev() {
            if !ProcessManager::is_pid_alive(pid) {
                continue;
            }
            let pid_s = pid.to_string();
            if let Err(e) = run_taskkill(&["/F", "/T", "/PID", &pid_s], pid) {
                last_err = Some(e);
            }
        }
        if ProcessManager::any_pid_alive(tree) || ProcessManager::is_pid_alive(root) {
            if let Some(e) = last_err {
                return Err(e);
            }
            return Err(ProcessError::Other(format!(
                "taskkill left processes alive for root {root}"
            )));
        }
        Ok(())
    }
    #[cfg(unix)]
    {
        // SIGKILL process group, then every descendant (deepest first), then root.
        unix_kill_group(root, libc::SIGKILL);
        for &pid in tree.iter().rev() {
            unix_kill_one(pid, libc::SIGKILL);
        }
        unix_kill_one(root, libc::SIGKILL);

        // One more pass for anything still listed as alive.
        let leftovers: Vec<u32> = tree
            .iter()
            .copied()
            .chain(std::iter::once(root))
            .filter(|p| ProcessManager::is_pid_alive(*p))
            .collect();
        for pid in leftovers.iter().rev() {
            unix_kill_group(*pid, libc::SIGKILL);
            unix_kill_one(*pid, libc::SIGKILL);
        }

        if ProcessManager::any_pid_alive(&leftovers) || ProcessManager::is_pid_alive(root) {
            let still: Vec<u32> = leftovers
                .into_iter()
                .filter(|p| ProcessManager::is_pid_alive(*p))
                .collect();
            return Err(ProcessError::Other(format!(
                "kill -9 left processes alive for root {root}: {still:?}"
            )));
        }
        Ok(())
    }
}

#[cfg(unix)]
fn unix_kill_group(pid: u32, sig: i32) -> bool {
    // Negative pid => entire process group (session leader after setsid).
    let rc = unsafe { libc::kill(-(pid as i32), sig) };
    rc == 0 || std::io::Error::last_os_error().raw_os_error() == Some(libc::ESRCH)
}

#[cfg(unix)]
fn unix_kill_one(pid: u32, sig: i32) -> bool {
    let rc = unsafe { libc::kill(pid as i32, sig) };
    rc == 0 || std::io::Error::last_os_error().raw_os_error() == Some(libc::ESRCH)
}

/// Append a marker line to log (useful for debugging).
#[allow(dead_code)]
pub fn append_marker(path: &Path, line: &str) -> Result<(), ProcessError> {
    let mut f = fs::OpenOptions::new().create(true).append(true).open(path)?;
    writeln!(f, "{line}")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn decode_log_bytes_utf8() {
        assert_eq!(decode_log_bytes(b"hello"), "hello");
    }

    #[test]
    fn resolve_work_dir_uses_explicit() {
        let dir = tempdir().unwrap();
        let path = dir.path().to_path_buf();
        let got = resolve_work_dir(Some(path.to_str().unwrap()), None).unwrap();
        assert_eq!(got, path);
    }

    #[test]
    fn tree_pids_include_root() {
        let pids = ProcessManager::collect_tree_pids(std::process::id());
        assert!(pids.contains(&std::process::id()));
    }

    #[test]
    fn persist_and_read_token_override() {
        let dir = tempdir().unwrap();
        let mut cfg = crate::config::Config::default();
        cfg.data_dir = dir.path().to_path_buf();
        cfg.persist_token("secret-xyz").unwrap();
        let text = std::fs::read_to_string(cfg.token_file_path()).unwrap();
        assert!(text.contains("secret-xyz"));
    }

    #[test]
    fn write_marker() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("x.log");
        append_marker(&path, "marker").unwrap();
        let mut f = std::fs::OpenOptions::new().append(true).open(&path).unwrap();
        writeln!(f, "line").unwrap();
        let s = std::fs::read_to_string(&path).unwrap();
        assert!(s.contains("marker"));
    }
}
