//! Audit entries with hash linking.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

/// Event severity level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EventSeverity {
    Debug,
    Info,
    Warning,
    Error,
    Critical,
    Security,
}

/// A single audit entry in the chain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    pub id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub severity: EventSeverity,
    pub source: String,
    pub action: String,
    pub details: serde_json::Value,
    pub agent_id: Option<String>,
    /// SHA-256 hash of the previous entry (empty string for genesis).
    pub prev_hash: String,
    /// SHA-256 hash of this entry (computed on creation).
    pub hash: String,
}

impl AuditEntry {
    /// Create a new entry chained to the given previous hash.
    pub fn new(
        severity: EventSeverity,
        source: impl Into<String>,
        action: impl Into<String>,
        details: serde_json::Value,
        prev_hash: impl Into<String>,
    ) -> Self {
        let mut entry = Self {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            severity,
            source: source.into(),
            action: action.into(),
            details,
            agent_id: None,
            prev_hash: prev_hash.into(),
            hash: String::new(),
        };
        entry.hash = entry.compute_hash();
        entry
    }

    pub fn with_agent(mut self, agent_id: impl Into<String>) -> Self {
        self.agent_id = Some(agent_id.into());
        self.hash = self.compute_hash();
        self
    }

    /// Compute the SHA-256 hash of this entry's content.
    pub fn compute_hash(&self) -> String {
        let mut hasher = Sha256::new();
        hasher.update(self.id.as_bytes());
        hasher.update(self.timestamp.to_rfc3339().as_bytes());
        hasher.update(format!("{:?}", self.severity).as_bytes());
        hasher.update(self.source.as_bytes());
        hasher.update(self.action.as_bytes());
        hasher.update(self.details.to_string().as_bytes());
        hasher.update(self.agent_id.as_deref().unwrap_or("").as_bytes());
        hasher.update(self.prev_hash.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// Verify this entry's hash matches its content.
    pub fn verify(&self) -> bool {
        self.hash == self.compute_hash()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entry_creation() {
        let entry = AuditEntry::new(
            EventSeverity::Info,
            "daimon",
            "agent.registered",
            serde_json::json!({"agent_id": "a1"}),
            "",
        );
        assert!(!entry.hash.is_empty());
        assert!(entry.verify());
    }

    #[test]
    fn entry_tamper_detection() {
        let mut entry = AuditEntry::new(
            EventSeverity::Security,
            "aegis",
            "policy.violation",
            serde_json::json!({}),
            "",
        );
        let original_hash = entry.hash.clone();
        entry.action = "tampered".into();
        assert_ne!(entry.compute_hash(), original_hash);
        assert!(!entry.verify());
    }

    #[test]
    fn entry_chaining() {
        let e1 = AuditEntry::new(EventSeverity::Info, "src", "act", serde_json::json!({}), "");
        let e2 = AuditEntry::new(
            EventSeverity::Info,
            "src",
            "act2",
            serde_json::json!({}),
            &e1.hash,
        );
        assert_eq!(e2.prev_hash, e1.hash);
        assert!(e2.verify());
    }

    #[test]
    fn entry_with_agent() {
        let entry = AuditEntry::new(EventSeverity::Info, "src", "act", serde_json::json!({}), "")
            .with_agent("agent-123");
        assert_eq!(entry.agent_id.as_deref(), Some("agent-123"));
        assert!(entry.verify());
    }

    #[test]
    fn serde_roundtrip() {
        let entry = AuditEntry::new(
            EventSeverity::Critical,
            "phylax",
            "threat.detected",
            serde_json::json!({"file": "/tmp/bad"}),
            "abc123",
        );
        let json = serde_json::to_string(&entry).unwrap();
        let back: AuditEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(back.hash, entry.hash);
        assert!(back.verify());
    }
}
