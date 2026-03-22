//! The audit chain — append-only, hash-linked sequence of entries.

use crate::LibroError;
use crate::entry::{AuditEntry, EventSeverity};

/// An append-only audit chain with hash-linked entries.
#[derive(Debug, Default)]
pub struct AuditChain {
    entries: Vec<AuditEntry>,
}

impl AuditChain {
    pub fn new() -> Self {
        Self::default()
    }

    /// Append an event to the chain. Automatically links to the previous entry's hash.
    pub fn append(
        &mut self,
        severity: EventSeverity,
        source: impl Into<String>,
        action: impl Into<String>,
        details: serde_json::Value,
    ) -> &AuditEntry {
        let prev_hash = self
            .entries
            .last()
            .map(|e| e.hash.clone())
            .unwrap_or_default();
        let entry = AuditEntry::new(severity, source, action, details, prev_hash);
        self.entries.push(entry);
        self.entries.last().unwrap()
    }

    /// Number of entries in the chain.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Get all entries.
    pub fn entries(&self) -> &[AuditEntry] {
        &self.entries
    }

    /// Get the last entry's hash (chain head).
    pub fn head_hash(&self) -> Option<&str> {
        self.entries.last().map(|e| e.hash.as_str())
    }

    /// Verify the entire chain's integrity.
    pub fn verify(&self) -> crate::Result<()> {
        if self.entries.is_empty() {
            return Ok(());
        }

        // Verify genesis entry
        if !self.entries[0].prev_hash.is_empty() {
            return Err(LibroError::IntegrityViolation {
                index: 0,
                expected: "(empty)".into(),
                actual: self.entries[0].prev_hash.clone(),
            });
        }

        for (i, entry) in self.entries.iter().enumerate() {
            // Verify each entry's self-hash
            if !entry.verify() {
                return Err(LibroError::IntegrityViolation {
                    index: i,
                    expected: entry.compute_hash(),
                    actual: entry.hash.clone(),
                });
            }

            // Verify chain linkage (skip genesis)
            if i > 0 && entry.prev_hash != self.entries[i - 1].hash {
                return Err(LibroError::IntegrityViolation {
                    index: i,
                    expected: self.entries[i - 1].hash.clone(),
                    actual: entry.prev_hash.clone(),
                });
            }
        }

        Ok(())
    }

    /// Query entries by source.
    pub fn by_source(&self, source: &str) -> Vec<&AuditEntry> {
        self.entries.iter().filter(|e| e.source == source).collect()
    }

    /// Query entries by severity.
    pub fn by_severity(&self, severity: EventSeverity) -> Vec<&AuditEntry> {
        self.entries
            .iter()
            .filter(|e| e.severity == severity)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chain_append_and_verify() {
        let mut chain = AuditChain::new();
        chain.append(
            EventSeverity::Info,
            "daimon",
            "agent.start",
            serde_json::json!({}),
        );
        chain.append(
            EventSeverity::Info,
            "daimon",
            "agent.stop",
            serde_json::json!({}),
        );
        assert_eq!(chain.len(), 2);
        assert!(chain.verify().is_ok());
    }

    #[test]
    fn chain_detects_tamper() {
        let mut chain = AuditChain::new();
        chain.append(EventSeverity::Info, "src", "act", serde_json::json!({}));
        chain.append(EventSeverity::Info, "src", "act2", serde_json::json!({}));

        // Tamper with first entry
        chain.entries[0].action = "hacked".into();
        assert!(chain.verify().is_err());
    }

    #[test]
    fn chain_query() {
        let mut chain = AuditChain::new();
        chain.append(
            EventSeverity::Info,
            "daimon",
            "start",
            serde_json::json!({}),
        );
        chain.append(
            EventSeverity::Security,
            "aegis",
            "alert",
            serde_json::json!({}),
        );
        chain.append(EventSeverity::Info, "daimon", "stop", serde_json::json!({}));

        assert_eq!(chain.by_source("daimon").len(), 2);
        assert_eq!(chain.by_severity(EventSeverity::Security).len(), 1);
    }

    #[test]
    fn empty_chain_valid() {
        let chain = AuditChain::new();
        assert!(chain.verify().is_ok());
        assert!(chain.is_empty());
    }

    #[test]
    fn head_hash() {
        let mut chain = AuditChain::new();
        assert!(chain.head_hash().is_none());
        chain.append(EventSeverity::Info, "src", "act", serde_json::json!({}));
        assert!(chain.head_hash().is_some());
    }
}
