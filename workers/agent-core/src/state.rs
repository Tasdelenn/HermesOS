//! Persistent worker credential state (ADR-014).
//!
//! After a successful enrollment the Brain issues a per-worker `worker_secret`.
//! The worker stores it in a small JSON state file (git-ignored, owner-only on
//! Unix) and presents it in every later `hello`.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkerState {
    pub worker_id: String,
    pub worker_secret: String,
}

// Never print the secret, even in debug output.
impl std::fmt::Debug for WorkerState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WorkerState")
            .field("worker_id", &self.worker_id)
            .field("worker_secret", &"<redacted>")
            .finish()
    }
}

/// Default state path: next to the config file, `<stem>.state.json`
/// (e.g. `config/worker.conf` -> `config/worker.state.json`).
pub fn default_state_path(config_path: &Path) -> PathBuf {
    let stem = config_path
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "worker".to_string());
    config_path.with_file_name(format!("{stem}.state.json"))
}

/// Load state if the file exists. `Ok(None)` means "not enrolled yet".
pub fn load(path: &Path) -> Result<Option<WorkerState>, String> {
    if !path.exists() {
        return Ok(None);
    }
    let raw = std::fs::read_to_string(path)
        .map_err(|e| format!("cannot read state file {}: {e}", path.display()))?;
    serde_json::from_str(&raw)
        .map(Some)
        .map_err(|e| format!("cannot parse state file {}: {e}", path.display()))
}

/// Atomically write state (temp file + rename), owner-only on Unix.
pub fn save(path: &Path, state: &WorkerState) -> Result<(), String> {
    use std::io::Write;

    let json = serde_json::to_string_pretty(state).map_err(|e| e.to_string())?;
    let err = |e: std::io::Error| format!("cannot write state file {}: {e}", path.display());

    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent).map_err(err)?;
        }
    }
    let tmp = path.with_extension("json.tmp");
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(&tmp).map_err(err)?;
    file.write_all(json.as_bytes()).map_err(err)?;
    file.sync_all().map_err(err)?;
    drop(file);
    std::fs::rename(&tmp, path).map_err(err)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir() -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "hermes-worker-state-{}-{nanos}",
            std::process::id()
        ))
    }

    #[test]
    fn default_path_is_next_to_config() {
        assert_eq!(
            default_state_path(Path::new("config/worker.conf")),
            PathBuf::from("config/worker.state.json")
        );
        assert_eq!(
            default_state_path(Path::new("/etc/hermes/pi.conf")),
            PathBuf::from("/etc/hermes/pi.state.json")
        );
    }

    #[test]
    fn missing_file_means_not_enrolled() {
        let dir = temp_dir();
        assert_eq!(load(&dir.join("nope.state.json")).unwrap(), None);
    }

    #[test]
    fn save_then_load_round_trips_with_private_permissions() {
        let dir = temp_dir();
        let path = dir.join("w.state.json");
        let state = WorkerState {
            worker_id: "w1".into(),
            worker_secret: "s3cret".into(),
        };
        save(&path, &state).unwrap();
        assert_eq!(load(&path).unwrap(), Some(state));

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(&path).unwrap().permissions().mode();
            assert_eq!(mode & 0o777, 0o600);
        }
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn debug_output_redacts_secret() {
        let state = WorkerState {
            worker_id: "w1".into(),
            worker_secret: "s3cret".into(),
        };
        let printed = format!("{state:?}");
        assert!(!printed.contains("s3cret"));
    }
}
