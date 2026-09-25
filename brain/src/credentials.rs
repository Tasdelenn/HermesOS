//! Per-worker credentials issued at enrollment (ADR-014).
//!
//! At enrollment the Brain generates a random 256-bit `worker_secret`, returns it
//! to the worker exactly once (in `enrollment_accepted`) and stores only its
//! SHA-256 digest keyed by `worker_id`. A later `hello` is accepted only if the
//! presented secret hashes to the stored digest (constant-time comparison).
//!
//! Storage: in memory, optionally mirrored to a JSON file (`--credentials-file`,
//! default `config/brain_credentials.json`, git-ignored). The file contains
//! digests only, never plaintext secrets, and is written atomically with
//! owner-only permissions on Unix. Without a file, all workers must re-enroll
//! after a Brain restart.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;
use tokio::sync::Mutex;

const SECRET_BYTES: usize = 32;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CredentialRecord {
    /// Hex-encoded SHA-256 digest of the worker secret.
    pub secret_sha256: String,
    /// Role declared at enrollment (informational for now).
    pub role: String,
    pub enrolled_at_ms: u64,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct CredentialFile {
    workers: HashMap<String, CredentialRecord>,
}

/// Outcome of verifying a `hello`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerifyResult {
    Ok,
    UnknownWorker,
    BadSecret,
}

#[derive(Debug, Clone, Default)]
pub struct CredentialStore {
    inner: Arc<Mutex<HashMap<String, CredentialRecord>>>,
    path: Option<Arc<PathBuf>>,
}

impl CredentialStore {
    /// Purely in-memory store (used by tests and when persistence is disabled).
    pub fn in_memory() -> Self {
        Self::default()
    }

    /// Load (or create on first write) a JSON-backed store at `path`.
    pub fn with_file(path: impl Into<PathBuf>) -> Result<Self, String> {
        let path = path.into();
        let workers = if path.exists() {
            let raw = std::fs::read_to_string(&path)
                .map_err(|e| format!("cannot read {}: {e}", path.display()))?;
            serde_json::from_str::<CredentialFile>(&raw)
                .map_err(|e| format!("cannot parse {}: {e}", path.display()))?
                .workers
        } else {
            HashMap::new()
        };
        Ok(Self {
            inner: Arc::new(Mutex::new(workers)),
            path: Some(Arc::new(path)),
        })
    }

    /// Issue (or rotate) the credential for `worker_id`. Returns the plaintext
    /// secret, which must be sent to the worker once and never stored.
    pub async fn issue(&self, worker_id: &str, role: &str) -> Result<String, String> {
        let secret = generate_secret()?;
        let record = CredentialRecord {
            secret_sha256: sha256_hex(&secret),
            role: role.to_string(),
            enrolled_at_ms: now_ms(),
        };
        let mut map = self.inner.lock().await;
        let previous = map.insert(worker_id.to_string(), record);
        if let Err(e) = self.persist(&map) {
            // Roll back so memory and disk stay consistent.
            match previous {
                Some(prev) => map.insert(worker_id.to_string(), prev),
                None => map.remove(worker_id),
            };
            return Err(e);
        }
        Ok(secret)
    }

    pub async fn verify(&self, worker_id: &str, secret: &str) -> VerifyResult {
        let map = self.inner.lock().await;
        match map.get(worker_id) {
            None => VerifyResult::UnknownWorker,
            Some(record) => {
                let presented = sha256_hex(secret);
                if bool::from(presented.as_bytes().ct_eq(record.secret_sha256.as_bytes())) {
                    VerifyResult::Ok
                } else {
                    VerifyResult::BadSecret
                }
            }
        }
    }

    pub async fn contains(&self, worker_id: &str) -> bool {
        self.inner.lock().await.contains_key(worker_id)
    }

    pub async fn len(&self) -> usize {
        self.inner.lock().await.len()
    }

    pub async fn is_empty(&self) -> bool {
        self.inner.lock().await.is_empty()
    }

    fn persist(&self, map: &HashMap<String, CredentialRecord>) -> Result<(), String> {
        let Some(path) = self.path.as_deref() else {
            return Ok(());
        };
        let file = CredentialFile {
            workers: map.clone(),
        };
        let json = serde_json::to_string_pretty(&file).map_err(|e| e.to_string())?;
        write_private_atomic(path, json.as_bytes())
            .map_err(|e| format!("cannot write {}: {e}", path.display()))
    }
}

fn generate_secret() -> Result<String, String> {
    let mut buf = [0u8; SECRET_BYTES];
    getrandom::fill(&mut buf).map_err(|e| format!("secure random unavailable: {e}"))?;
    Ok(to_hex(&buf))
}

fn sha256_hex(input: &str) -> String {
    to_hex(&Sha256::digest(input.as_bytes()))
}

fn to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

/// Write `data` to `path` via a temp file + rename, owner-only on Unix.
fn write_private_atomic(path: &Path, data: &[u8]) -> std::io::Result<()> {
    use std::io::Write;

    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
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
    let mut file = options.open(&tmp)?;
    file.write_all(data)?;
    file.sync_all()?;
    drop(file);
    std::fs::rename(&tmp, path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn issue_then_verify() {
        let store = CredentialStore::in_memory();
        let secret = store.issue("w1", "dev").await.unwrap();
        assert_eq!(secret.len(), SECRET_BYTES * 2);
        assert_eq!(store.verify("w1", &secret).await, VerifyResult::Ok);
        assert_eq!(store.verify("w1", "nope").await, VerifyResult::BadSecret);
        assert_eq!(
            store.verify("w2", &secret).await,
            VerifyResult::UnknownWorker
        );
    }

    #[tokio::test]
    async fn reissue_rotates_secret() {
        let store = CredentialStore::in_memory();
        let old = store.issue("w1", "dev").await.unwrap();
        let new = store.issue("w1", "dev").await.unwrap();
        assert_ne!(old, new);
        assert_eq!(store.verify("w1", &old).await, VerifyResult::BadSecret);
        assert_eq!(store.verify("w1", &new).await, VerifyResult::Ok);
    }

    #[tokio::test]
    async fn file_store_persists_only_digests() {
        let dir = std::env::temp_dir().join(format!("hermes-cred-test-{}", uuid::Uuid::new_v4()));
        let path = dir.join("creds.json");

        let store = CredentialStore::with_file(&path).unwrap();
        let secret = store.issue("w1", "dev").await.unwrap();

        let raw = std::fs::read_to_string(&path).unwrap();
        assert!(
            !raw.contains(&secret),
            "plaintext secret must not be stored"
        );
        assert!(raw.contains(&sha256_hex(&secret)));

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(&path).unwrap().permissions().mode();
            assert_eq!(mode & 0o777, 0o600);
        }

        // Reload from disk: the credential survives a restart.
        let reloaded = CredentialStore::with_file(&path).unwrap();
        assert_eq!(reloaded.verify("w1", &secret).await, VerifyResult::Ok);

        std::fs::remove_dir_all(dir).ok();
    }
}
