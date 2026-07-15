use crate::models::{ServiceKind, ServiceSpec, ServiceState, ServiceStatus};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
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
}

pub struct ProcessManager {
    data_dir: PathBuf,
}

impl ProcessManager {
    pub fn new(data_dir: PathBuf) -> Self {
        let _ = fs::create_dir_all(data_dir.join("services"));
        Self { data_dir }
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
            env: std::collections::HashMap::new(),
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
        let meta = Self::meta_from_spec(spec)?;
        self.write_meta(&meta)?;
        Ok(self.status(&meta.name))
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

    pub fn status(&self, name: &str) -> ServiceStatus {
        let meta = self.read_meta(name).ok().flatten();
        let health_url = meta
            .as_ref()
            .and_then(|m| m.spec.as_ref())
            .and_then(|s| s.health_url.as_deref());
        match self.read_record(name) {
            Ok(Some(record)) if Self::is_pid_alive(record.pid) => {
                if meta.is_none() {
                    let _ = self.write_meta(&ServiceMeta {
                        name: name.to_string(),
                        kind: record.kind.clone(),
                        target: record.target.clone(),
                        spec: None,
                    });
                }
                let status = Self::status_running(name, &record);
                Self::apply_health(status, health_url)
            }
            Ok(Some(record)) => {
                let _ = self.clear_record(name);
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
                    });
                }
                Self::status_stopped(name, kind, target, Some("stale pid cleared".into()))
            }
            Ok(None) => {
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
        let current = self.status(name);
        if current.pid.is_some() {
            self.stop(name)?;
        }
        self.start_saved(name)
    }

    /// Remove definition (+ stop if running). Deletes meta and pid; keeps log file.
    pub fn remove(&self, name: &str) -> Result<ServiceStatus, ProcessError> {
        let current = self.status(name);
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
        Ok(names.iter().map(|n| self.status(n)).collect())
    }

    pub fn start(&self, spec: &ServiceSpec) -> Result<ServiceStatus, ProcessError> {
        let current = self.status(&spec.name);
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

        // Detach from agent process group on Unix; on Windows this is a no-op style spawn.
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            cmd.process_group(0);
        }

        let child: Child = cmd.spawn()?;
        let pid = child.id();
        let started_at = Utc::now();

        self.write_meta(&Self::meta_from_spec(spec)?)?;

        self.write_record(
            &spec.name,
            &PidRecord {
                pid,
                kind: Self::kind_str(kind).to_string(),
                target: target.clone(),
                started_at,
            },
        )?;

        // Avoid waiting on child; ownership dropped intentionally.
        std::mem::forget(child);

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
            // Give process a brief moment before first probe.
            std::thread::sleep(std::time::Duration::from_secs(2));
            status = Self::apply_health(status, Some(url));
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
        let record = self
            .read_record(name)?
            .ok_or(ProcessError::NotRunning)?;

        if !Self::is_pid_alive(record.pid) {
            self.clear_record(name)?;
            return Err(ProcessError::NotRunning);
        }

        // Soft stop is best-effort; many Windows console/Java processes ignore it.
        let _ = terminate_pid(record.pid);

        for _ in 0..30 {
            if !Self::is_pid_alive(record.pid) {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }

        if Self::is_pid_alive(record.pid) {
            force_kill_pid(record.pid)?;
            for _ in 0..20 {
                if !Self::is_pid_alive(record.pid) {
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
            if Self::is_pid_alive(record.pid) {
                return Err(ProcessError::Other(format!(
                    "process still alive after force kill (pid={})",
                    record.pid
                )));
            }
        }

        // Keep existing definition (incl. full spec); only clear runtime pid.
        if self.read_meta(name)?.is_none() {
            let _ = self.write_meta(&ServiceMeta {
                name: name.to_string(),
                kind: record.kind.clone(),
                target: record.target.clone(),
                spec: None,
            });
        }
        self.clear_record(name)?;

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

fn terminate_pid(pid: u32) -> Result<(), ProcessError> {
    #[cfg(windows)]
    {
        let pid_s = pid.to_string();
        // Soft terminate process tree; failure is OK — caller may force-kill.
        let _ = run_taskkill(&["/PID", &pid_s, "/T"], pid);
        Ok(())
    }
    #[cfg(unix)]
    {
        let status = Command::new("kill")
            .args(["-15", &pid.to_string()])
            .status()?;
        if !status.success() {
            return Err(ProcessError::Other(format!("kill -15 failed for pid {pid}")));
        }
        Ok(())
    }
}

fn force_kill_pid(pid: u32) -> Result<(), ProcessError> {
    #[cfg(windows)]
    {
        let pid_s = pid.to_string();
        run_taskkill(&["/F", "/T", "/PID", &pid_s], pid)
    }
    #[cfg(unix)]
    {
        let status = Command::new("kill")
            .args(["-9", &pid.to_string()])
            .status()?;
        if !status.success() {
            return Err(ProcessError::Other(format!("kill -9 failed for pid {pid}")));
        }
        Ok(())
    }
}

/// Append a marker line to log (useful for debugging).
#[allow(dead_code)]
pub fn append_marker(path: &Path, line: &str) -> Result<(), ProcessError> {
    let mut f = fs::OpenOptions::new().create(true).append(true).open(path)?;
    writeln!(f, "{line}")?;
    Ok(())
}
