use std::collections::HashSet;
use std::sync::Arc;

use tokio::sync::Mutex;

/// Manages single-use registration tokens.
///
/// In production this would be backed by a database. The prototype keeps
/// tokens in memory so they survive only for the lifetime of the process.
#[derive(Debug, Clone)]
pub struct EnrollmentService {
    /// Set of valid, unused tokens.
    tokens: Arc<Mutex<HashSet<String>>>,
}

impl EnrollmentService {
    pub fn new() -> Self {
        Self {
            tokens: Arc::new(Mutex::new(HashSet::new())),
        }
    }

    /// Pre-load tokens (e.g. from CLI flags or a config file).
    pub async fn add_token(&self, token: String) {
        self.tokens.lock().await.insert(token);
    }

    /// Validate and consume a registration token. Returns `true` if the token
    /// was valid and is now invalidated.
    pub async fn consume_token(&self, token: &str) -> bool {
        self.tokens.lock().await.remove(token)
    }

    pub async fn token_count(&self) -> usize {
        self.tokens.lock().await.len()
    }
}

impl Default for EnrollmentService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn consume_valid_token() {
        let svc = EnrollmentService::new();
        svc.add_token("tok-abc".into()).await;
        assert!(svc.consume_token("tok-abc").await);
        // second use must fail
        assert!(!svc.consume_token("tok-abc").await);
    }

    #[tokio::test]
    async fn reject_unknown_token() {
        let svc = EnrollmentService::new();
        assert!(!svc.consume_token("unknown").await);
    }

    #[tokio::test]
    async fn token_count() {
        let svc = EnrollmentService::new();
        svc.add_token("a".into()).await;
        svc.add_token("b".into()).await;
        assert_eq!(svc.token_count().await, 2);
        svc.consume_token("a").await;
        assert_eq!(svc.token_count().await, 1);
    }
}
