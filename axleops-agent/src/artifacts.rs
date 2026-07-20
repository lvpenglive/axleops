//! Versioned artifact store under `data_dir/artifacts/{service}/`.
//!
//! Layout:
//! ```text
//! artifacts/{service}/
//!   manifest.json
//!   {version_id}/{filename}
//! ```

use crate::models::{ServiceKind, ServiceSpec, ServiceStatus};
use crate::process::{ProcessError, ProcessManager};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

const MANIFEST: &str = "manifest.json";
const MAX_KEEP: usize = 12;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactInfo {
    pub id: String,
    pub filename: String,
    pub size: u64,
    pub uploaded_at: DateTime<Utc>,
    /// Absolute path on the agent host.
    pub path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    #[serde(default)]
    pub active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct Manifest {
    #[serde(default)]
    active: Option<String>,
    #[serde(default)]
    previous: Option<String>,
    #[serde(default)]
    versions: Vec<ArtifactRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ArtifactRecord {
    id: String,
    filename: String,
    size: u64,
    uploaded_at: DateTime<Utc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    note: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ArtifactList {
    pub service: String,
    pub active: Option<String>,
    pub previous: Option<String>,
    pub artifacts: Vec<ArtifactInfo>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PublishResult {
    pub service: String,
    pub version: String,
    pub previous: Option<String>,
    pub path: String,
    pub status: ServiceStatus,
}

pub struct ArtifactStore {
    root: PathBuf,
}

impl ArtifactStore {
    pub fn new(data_dir: &Path) -> Self {
        let root = data_dir.join("artifacts");
        let _ = fs::create_dir_all(&root);
        Self { root }
    }

    fn service_dir(&self, service: &str) -> Result<PathBuf, ProcessError> {
        let safe = sanitize_name(service)?;
        Ok(self.root.join(safe))
    }

    fn load_manifest(&self, service: &str) -> Result<(PathBuf, Manifest), ProcessError> {
        let dir = self.service_dir(service)?;
        fs::create_dir_all(&dir)?;
        let path = dir.join(MANIFEST);
        if !path.exists() {
            return Ok((dir, Manifest::default()));
        }
        let text = fs::read_to_string(&path)?;
        let m: Manifest = serde_json::from_str(&text)
            .map_err(|e| ProcessError::Other(format!("corrupt artifact manifest: {e}")))?;
        Ok((dir, m))
    }

    fn save_manifest(&self, dir: &Path, m: &Manifest) -> Result<(), ProcessError> {
        let path = dir.join(MANIFEST);
        let tmp = dir.join("manifest.json.tmp");
        let text = serde_json::to_string_pretty(m)
            .map_err(|e| ProcessError::Other(e.to_string()))?;
        fs::write(&tmp, text)?;
        fs::rename(&tmp, &path)?;
        Ok(())
    }

    pub fn list(&self, service: &str) -> Result<ArtifactList, ProcessError> {
        let (dir, m) = self.load_manifest(service)?;
        let artifacts = m
            .versions
            .iter()
            .rev()
            .map(|v| ArtifactInfo {
                id: v.id.clone(),
                filename: v.filename.clone(),
                size: v.size,
                uploaded_at: v.uploaded_at,
                path: dir.join(&v.id).join(&v.filename).display().to_string(),
                note: v.note.clone(),
                active: m.active.as_deref() == Some(v.id.as_str()),
            })
            .collect();
        Ok(ArtifactList {
            service: service.to_string(),
            active: m.active,
            previous: m.previous,
            artifacts,
        })
    }

    pub fn store(
        &self,
        service: &str,
        filename: &str,
        bytes: &[u8],
        version: Option<&str>,
        note: Option<&str>,
    ) -> Result<ArtifactInfo, ProcessError> {
        if bytes.is_empty() {
            return Err(ProcessError::Other("empty upload".into()));
        }
        let filename = sanitize_filename(filename)?;
        let id = match version {
            Some(v) if !v.trim().is_empty() => sanitize_name(v.trim())?,
            _ => default_version_id(),
        };

        let (dir, mut m) = self.load_manifest(service)?;
        if m.versions.iter().any(|v| v.id == id) {
            return Err(ProcessError::Other(format!(
                "artifact version already exists: {id}"
            )));
        }

        let ver_dir = dir.join(&id);
        fs::create_dir_all(&ver_dir)?;
        let file_path = ver_dir.join(&filename);
        {
            let mut f = fs::File::create(&file_path)?;
            f.write_all(bytes)?;
            f.sync_all()?;
        }

        let record = ArtifactRecord {
            id: id.clone(),
            filename: filename.clone(),
            size: bytes.len() as u64,
            uploaded_at: Utc::now(),
            note: note.map(|s| s.trim().to_string()).filter(|s| !s.is_empty()),
        };
        m.versions.push(record.clone());
        self.prune(&dir, &mut m)?;
        self.save_manifest(&dir, &m)?;

        Ok(ArtifactInfo {
            id: record.id,
            filename: record.filename,
            size: record.size,
            uploaded_at: record.uploaded_at,
            path: file_path.display().to_string(),
            note: record.note,
            active: false,
        })
    }

    fn prune(&self, dir: &Path, m: &mut Manifest) -> Result<(), ProcessError> {
        while m.versions.len() > MAX_KEEP {
            let oldest = m.versions.remove(0);
            if m.active.as_deref() == Some(oldest.id.as_str())
                || m.previous.as_deref() == Some(oldest.id.as_str())
            {
                // Keep active/previous even if over limit — put back and stop.
                m.versions.insert(0, oldest);
                break;
            }
            let _ = fs::remove_dir_all(dir.join(&oldest.id));
        }
        Ok(())
    }

    fn resolve_path(&self, dir: &Path, m: &Manifest, version: &str) -> Result<PathBuf, ProcessError> {
        let rec = m
            .versions
            .iter()
            .find(|v| v.id == version)
            .ok_or_else(|| ProcessError::Other(format!("artifact version not found: {version}")))?;
        let path = dir.join(&rec.id).join(&rec.filename);
        if !path.is_file() {
            return Err(ProcessError::Other(format!(
                "artifact file missing: {}",
                path.display()
            )));
        }
        Ok(path)
    }

    /// Point saved service target at an artifact (no stop/start).
    pub fn activate(
        &self,
        processes: &ProcessManager,
        service: &str,
        version: &str,
    ) -> Result<ArtifactInfo, ProcessError> {
        let (dir, mut m) = self.load_manifest(service)?;
        let path = self.resolve_path(&dir, &m, version)?;
        let path_str = path.display().to_string();

        let mut spec = processes.get_spec(service)?;
        apply_path_to_spec(&mut spec, &path_str)?;
        processes.save(&spec)?;

        if m.active.as_deref() != Some(version) {
            m.previous = m.active.clone();
            m.active = Some(version.to_string());
            self.save_manifest(&dir, &m)?;
        }

        let rec = m
            .versions
            .iter()
            .find(|v| v.id == version)
            .cloned()
            .unwrap();
        Ok(ArtifactInfo {
            id: rec.id,
            filename: rec.filename,
            size: rec.size,
            uploaded_at: rec.uploaded_at,
            path: path_str,
            note: rec.note,
            active: true,
        })
    }

    /// Stop → activate version → optional start.
    pub fn publish(
        &self,
        processes: &ProcessManager,
        service: &str,
        version: &str,
        start_after: bool,
    ) -> Result<PublishResult, ProcessError> {
        let (dir, m) = self.load_manifest(service)?;
        let path = self.resolve_path(&dir, &m, version)?;
        let path_str = path.display().to_string();
        let previous = m.active.clone();

        // Stop if running (ignore NotRunning).
        match processes.stop(service) {
            Ok(_) | Err(ProcessError::NotRunning) => {}
            Err(e) => return Err(e),
        }

        let info = self.activate(processes, service, version)?;

        let status = if start_after {
            processes.start_saved(service)?
        } else {
            processes.status(service)
        };

        Ok(PublishResult {
            service: service.to_string(),
            version: info.id,
            previous,
            path: path_str,
            status,
        })
    }

    /// Activate the previous version (swap) and optionally start.
    pub fn rollback(
        &self,
        processes: &ProcessManager,
        service: &str,
        start_after: bool,
    ) -> Result<PublishResult, ProcessError> {
        let (_, m) = self.load_manifest(service)?;
        let prev = m
            .previous
            .clone()
            .ok_or_else(|| ProcessError::Other("no previous artifact to roll back to".into()))?;
        self.publish(processes, service, &prev, start_after)
    }

    pub fn delete(&self, service: &str, version: &str) -> Result<(), ProcessError> {
        let (dir, mut m) = self.load_manifest(service)?;
        if m.active.as_deref() == Some(version) {
            return Err(ProcessError::Other(
                "cannot delete the active artifact; publish another version first".into(),
            ));
        }
        let before = m.versions.len();
        m.versions.retain(|v| v.id != version);
        if m.versions.len() == before {
            return Err(ProcessError::Other(format!(
                "artifact version not found: {version}"
            )));
        }
        if m.previous.as_deref() == Some(version) {
            m.previous = None;
        }
        let _ = fs::remove_dir_all(dir.join(version));
        self.save_manifest(&dir, &m)?;
        Ok(())
    }
}

fn apply_path_to_spec(spec: &mut ServiceSpec, path: &str) -> Result<(), ProcessError> {
    match spec.resolved_kind().map_err(ProcessError::Other)? {
        ServiceKind::Jar => {
            spec.jar_path = Some(path.to_string());
            spec.kind = Some(ServiceKind::Jar);
        }
        ServiceKind::Script => {
            spec.script_path = Some(path.to_string());
            spec.kind = Some(ServiceKind::Script);
        }
        ServiceKind::Command => {
            return Err(ProcessError::Other(
                "artifact publish is only supported for jar and script services".into(),
            ));
        }
    }
    Ok(())
}

fn sanitize_name(name: &str) -> Result<String, ProcessError> {
    let s: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.' {
                c
            } else {
                '_'
            }
        })
        .collect();
    if s.is_empty() || s == "." || s == ".." {
        return Err(ProcessError::Other("invalid name".into()));
    }
    if s.contains("..") {
        return Err(ProcessError::Other("invalid name".into()));
    }
    Ok(s)
}

fn sanitize_filename(name: &str) -> Result<String, ProcessError> {
    let base = Path::new(name)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .trim();
    if base.is_empty() {
        return Err(ProcessError::Other("filename required".into()));
    }
    sanitize_name(base)
}

fn default_version_id() -> String {
    let ts = Utc::now().format("%Y%m%d-%H%M%S");
    let short = &uuid::Uuid::new_v4().simple().to_string()[..8];
    format!("{ts}-{short}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use tempfile::tempdir;

    fn mgr(dir: &Path) -> ProcessManager {
        let mut cfg = Config::default();
        cfg.data_dir = dir.to_path_buf();
        let _ = fs::create_dir_all(dir.join("services"));
        ProcessManager::new(&cfg)
    }

    #[test]
    fn store_list_activate_publish() {
        let dir = tempdir().unwrap();
        let store = ArtifactStore::new(dir.path());
        let processes = mgr(dir.path());

        let spec = ServiceSpec {
            name: "demo".into(),
            kind: Some(ServiceKind::Jar),
            jar_path: Some("/tmp/old.jar".into()),
            script_path: None,
            command: None,
            interpreter: None,
            work_dir: None,
            jvm_args: vec![],
            app_args: vec![],
            args: vec![],
            env: Default::default(),
            health_url: None,
        };
        processes.save(&spec).unwrap();

        let a = store
            .store("demo", "app.jar", b"jar-bytes-v1", Some("v1"), Some("first"))
            .unwrap();
        assert_eq!(a.id, "v1");
        let list = store.list("demo").unwrap();
        assert_eq!(list.artifacts.len(), 1);

        store.activate(&processes, "demo", "v1").unwrap();
        let spec2 = processes.get_spec("demo").unwrap();
        let jar = spec2.jar_path.unwrap();
        assert!(jar.contains("v1") && jar.ends_with("app.jar"), "jar={jar}");

        let _ = store
            .store("demo", "app.jar", b"jar-bytes-v2", Some("v2"), None)
            .unwrap();
        let pubres = store.publish(&processes, "demo", "v2", false).unwrap();
        assert_eq!(pubres.version, "v2");
        assert_eq!(pubres.previous.as_deref(), Some("v1"));

        let rb = store.rollback(&processes, "demo", false).unwrap();
        assert_eq!(rb.version, "v1");
    }
}
