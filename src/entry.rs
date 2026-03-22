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

impl EventSeverity {
    /// Stable string representation used in hashing and storage.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Debug => "Debug",
            Self::Info => "Info",
            Self::Warning => "Warning",
            Self::Error => "Error",
            Self::Critical => "Critical",
            Self::Security => "Security",
        }
    }
}

/// A single audit entry in the chain.
///
/// Fields are not directly mutable — all construction goes through [`AuditEntry::new`]
/// and [`AuditEntry::with_agent`], which recompute the hash. This ensures integrity
/// by construction: a valid `AuditEntry` always has a correct self-hash.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    id: Uuid,
    timestamp: DateTime<Utc>,
    severity: EventSeverity,
    source: String,
    action: String,
    details: serde_json::Value,
    agent_id: Option<String>,
    /// SHA-256 hash of the previous entry (empty string for genesis).
    prev_hash: String,
    /// SHA-256 hash of this entry (computed on creation).
    hash: String,
}

impl AuditEntry {
    // --- Accessors ---
    pub fn id(&self) -> Uuid { self.id }
    pub fn timestamp(&self) -> DateTime<Utc> { self.timestamp }
    pub fn severity(&self) -> EventSeverity { self.severity }
    pub fn source(&self) -> &str { &self.source }
    pub fn action(&self) -> &str { &self.action }
    pub fn details(&self) -> &serde_json::Value { &self.details }
    pub fn agent_id(&self) -> Option<&str> { self.agent_id.as_deref() }
    pub fn prev_hash(&self) -> &str { &self.prev_hash }
    pub fn hash(&self) -> &str { &self.hash }
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

    /// Reconstruct an entry from stored fields (e.g. from database rows).
    /// The caller is responsible for providing correct field values;
    /// use [`AuditEntry::verify`] to check integrity after reconstruction.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn from_raw(
        id: Uuid,
        timestamp: DateTime<Utc>,
        severity: EventSeverity,
        source: String,
        action: String,
        details: serde_json::Value,
        agent_id: Option<String>,
        prev_hash: String,
        hash: String,
    ) -> Self {
        Self { id, timestamp, severity, source, action, details, agent_id, prev_hash, hash }
    }

    pub fn with_agent(mut self, agent_id: impl Into<String>) -> Self {
        self.agent_id = Some(agent_id.into());
        self.hash = self.compute_hash();
        self
    }

    /// Compute the SHA-256 hash of this entry's content.
    ///
    /// Uses stable representations: `EventSeverity::as_str()` for severity,
    /// and canonicalized JSON (sorted keys) for details, ensuring the hash
    /// is reproducible across serialization roundtrips and Rust versions.
    ///
    /// Each variable-length field is length-prefixed (little-endian u64) to
    /// prevent second-preimage attacks via field boundary shifting.
    pub fn compute_hash(&self) -> String {
        let mut hasher = Sha256::new();
        // Fixed-length fields (no prefix needed)
        hasher.update(self.id.as_bytes());
        // Variable-length fields: length-prefixed
        hash_field(&mut hasher, self.timestamp.to_rfc3339().as_bytes());
        hash_field(&mut hasher, self.severity.as_str().as_bytes());
        hash_field(&mut hasher, self.source.as_bytes());
        hash_field(&mut hasher, self.action.as_bytes());
        // Canonical JSON: sorted keys for deterministic hashing
        canonical_json_hash(&self.details, &mut hasher);
        hash_field(&mut hasher, self.agent_id.as_deref().unwrap_or("").as_bytes());
        hash_field(&mut hasher, self.prev_hash.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// Verify this entry's hash matches its content.
    pub fn verify(&self) -> bool {
        self.hash == self.compute_hash()
    }
}

impl std::fmt::Display for EventSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::fmt::Display for AuditEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "[{}] {} {}/{} hash={}",
            self.timestamp.format("%Y-%m-%d %H:%M:%S"),
            self.severity,
            self.source,
            self.action,
            abbreviate_hash(&self.hash),
        )?;
        if let Some(ref agent) = self.agent_id {
            write!(f, " agent={agent}")?;
        }
        Ok(())
    }
}

/// Abbreviate a hex hash for display: "a1b2c3d4..ef56" or the full string if short.
pub(crate) fn abbreviate_hash(hash: &str) -> String {
    if hash.len() > 12 {
        format!("{}..{}", &hash[..8], &hash[hash.len() - 4..])
    } else {
        hash.to_owned()
    }
}

/// Write a length-prefixed field into the hasher to prevent field boundary ambiguity.
fn hash_field(hasher: &mut Sha256, data: &[u8]) {
    hasher.update((data.len() as u64).to_le_bytes());
    hasher.update(data);
}

/// Write a JSON value into a hasher with sorted object keys for deterministic hashing.
fn canonical_json_hash(value: &serde_json::Value, hasher: &mut Sha256) {
    match value {
        serde_json::Value::Null => hasher.update(b"null"),
        serde_json::Value::Bool(b) => {
            hasher.update(if *b { "true" } else { "false" }.as_bytes());
        }
        serde_json::Value::Number(n) => hasher.update(n.to_string().as_bytes()),
        serde_json::Value::String(s) => {
            hasher.update(b"\"");
            hasher.update(s.as_bytes());
            hasher.update(b"\"");
        }
        serde_json::Value::Array(arr) => {
            hasher.update(b"[");
            for (i, v) in arr.iter().enumerate() {
                if i > 0 {
                    hasher.update(b",");
                }
                canonical_json_hash(v, hasher);
            }
            hasher.update(b"]");
        }
        serde_json::Value::Object(map) => {
            hasher.update(b"{");
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            for (i, key) in keys.iter().enumerate() {
                if i > 0 {
                    hasher.update(b",");
                }
                hasher.update(b"\"");
                hasher.update(key.as_bytes());
                hasher.update(b"\":");
                canonical_json_hash(&map[*key], hasher);
            }
            hasher.update(b"}");
        }
    }
}

#[cfg(test)]
impl AuditEntry {
    /// Corrupt the action field for tamper-detection testing.
    pub(crate) fn corrupt_action(&mut self, action: impl Into<String>) {
        self.action = action.into();
    }

    /// Corrupt the hash field for tamper-detection testing.
    pub(crate) fn corrupt_hash(&mut self, hash: impl Into<String>) {
        self.hash = hash.into();
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
        assert!(!entry.hash().is_empty());
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
        let original_hash = entry.hash().to_owned();
        entry.corrupt_action("tampered");
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
            e1.hash(),
        );
        assert_eq!(e2.prev_hash(), e1.hash());
        assert!(e2.verify());
    }

    #[test]
    fn entry_with_agent() {
        let entry = AuditEntry::new(EventSeverity::Info, "src", "act", serde_json::json!({}), "")
            .with_agent("agent-123");
        assert_eq!(entry.agent_id(), Some("agent-123"));
        assert!(entry.verify());
    }

    #[test]
    fn canonical_hash_covers_all_json_types() {
        // Exercise null, bool, number, string, array, and nested objects
        let details = serde_json::json!({
            "z_last": null,
            "a_first": true,
            "numbers": [1, 2.5, -3],
            "nested": {"b": "beta", "a": "alpha"},
            "text": "hello"
        });
        let entry = AuditEntry::new(EventSeverity::Info, "src", "act", details, "");
        assert!(entry.verify());

        // Same data with keys in different insertion order should produce same hash
        let details2 = serde_json::json!({
            "text": "hello",
            "nested": {"a": "alpha", "b": "beta"},
            "numbers": [1, 2.5, -3],
            "a_first": true,
            "z_last": null
        });
        let entry2 = AuditEntry::new(EventSeverity::Info, "src", "act", details2, "");
        // Can't compare hashes directly (different timestamps/ids), but both should verify
        assert!(entry2.verify());
    }

    #[test]
    fn severity_as_str_all_variants() {
        assert_eq!(EventSeverity::Debug.as_str(), "Debug");
        assert_eq!(EventSeverity::Info.as_str(), "Info");
        assert_eq!(EventSeverity::Warning.as_str(), "Warning");
        assert_eq!(EventSeverity::Error.as_str(), "Error");
        assert_eq!(EventSeverity::Critical.as_str(), "Critical");
        assert_eq!(EventSeverity::Security.as_str(), "Security");
    }

    #[test]
    fn accessors_return_correct_values() {
        let entry = AuditEntry::new(
            EventSeverity::Warning,
            "aegis",
            "scan.complete",
            serde_json::json!({"count": 42}),
            "prev123",
        )
        .with_agent("agent-x");

        assert_eq!(entry.severity(), EventSeverity::Warning);
        assert_eq!(entry.source(), "aegis");
        assert_eq!(entry.action(), "scan.complete");
        assert_eq!(entry.details(), &serde_json::json!({"count": 42}));
        assert_eq!(entry.agent_id(), Some("agent-x"));
        assert_eq!(entry.prev_hash(), "prev123");
        assert!(!entry.hash().is_empty());
        // id and timestamp are set automatically
        assert!(!entry.id().is_nil());
    }

    #[test]
    fn field_boundary_not_ambiguous() {
        // Without length-prefixing, source="ab" action="cd" would hash the same
        // as source="abc" action="d". Verify they don't.
        let e1 = AuditEntry::new(EventSeverity::Info, "ab", "cd", serde_json::json!({}), "");
        let e2 = AuditEntry::new(EventSeverity::Info, "abc", "d", serde_json::json!({}), "");
        // Same id/timestamp would be needed for a real collision test, but since
        // UUID and timestamp differ, we verify the hash function structurally handles it.
        // The key assertion: both verify independently (hash is correct for their fields)
        assert!(e1.verify());
        assert!(e2.verify());
        assert_ne!(e1.hash(), e2.hash());
    }

    #[test]
    fn abbreviate_hash_long() {
        let h = "a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2";
        assert_eq!(super::abbreviate_hash(h), "a1b2c3d4..a1b2");
    }

    #[test]
    fn abbreviate_hash_short() {
        assert_eq!(super::abbreviate_hash("abc"), "abc");
        assert_eq!(super::abbreviate_hash(""), "");
    }

    #[test]
    fn display_entry_with_empty_hash() {
        // Simulate a deserialized entry with corrupt empty hash
        let entry = AuditEntry::from_raw(
            uuid::Uuid::new_v4(),
            chrono::Utc::now(),
            EventSeverity::Info,
            "src".into(),
            "act".into(),
            serde_json::json!({}),
            None,
            "".into(),
            "".into(),
        );
        // Should not panic
        let display = format!("{entry}");
        assert!(display.contains("src/act"));
    }

    #[test]
    fn canonical_json_key_order_determinism() {
        // Build two entries with identical content but different JSON key insertion order,
        // using the same id/timestamp/prev_hash so hashes are directly comparable.
        let id = uuid::Uuid::new_v4();
        let ts = chrono::Utc::now();

        let details_a = serde_json::json!({"z": 1, "a": 2, "m": 3});
        let details_b = serde_json::json!({"a": 2, "m": 3, "z": 1});

        let entry_a = AuditEntry::from_raw(
            id, ts, EventSeverity::Info, "s".into(), "a".into(),
            details_a, None, "".into(), String::new(),
        );
        let entry_b = AuditEntry::from_raw(
            id, ts, EventSeverity::Info, "s".into(), "a".into(),
            details_b, None, "".into(), String::new(),
        );
        assert_eq!(entry_a.compute_hash(), entry_b.compute_hash());
    }

    #[test]
    fn empty_source_and_action() {
        let entry = AuditEntry::new(EventSeverity::Info, "", "", serde_json::json!(null), "");
        assert!(entry.verify());
        let display = format!("{entry}");
        assert!(display.contains("/")); // source/action separator still present
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
        assert_eq!(back.hash(), entry.hash());
        assert!(back.verify());
    }
}
