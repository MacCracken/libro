//! The audit chain — append-only, hash-linked sequence of entries.

use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

use crate::LibroError;
use crate::entry::{AuditEntry, EventSeverity};
use crate::query::QueryFilter;
use crate::retention::RetentionPolicy;
use crate::verify::verify_chain;

/// An append-only audit chain with hash-linked entries.
#[derive(Debug, Default)]
pub struct AuditChain {
    pub(crate) entries: Vec<AuditEntry>,
    /// After rotation, holds the head hash of the previous chain for continuity.
    pub(crate) prev_chain_hash: Option<String>,
}

/// Archived entries from a chain rotation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ChainArchive {
    /// The drained entries.
    pub entries: Vec<AuditEntry>,
    /// The head hash of the archived chain (used to link the next chain).
    pub head_hash: String,
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
            .map(|e| e.hash().to_owned())
            .or_else(|| self.prev_chain_hash.clone())
            .unwrap_or_default();
        let entry = AuditEntry::new(severity, source, action, details, prev_hash);
        debug!(
            hash = entry.hash(),
            source = entry.source(),
            action = entry.action(),
            severity = entry.severity().as_str(),
            index = self.entries.len(),
            "chain entry appended"
        );
        self.entries.push(entry);
        self.entries.last().unwrap()
    }

    /// Append an event with an agent ID to the chain.
    pub fn append_with_agent(
        &mut self,
        severity: EventSeverity,
        source: impl Into<String>,
        action: impl Into<String>,
        details: serde_json::Value,
        agent_id: impl Into<String>,
    ) -> &AuditEntry {
        let prev_hash = self
            .entries
            .last()
            .map(|e| e.hash().to_owned())
            .or_else(|| self.prev_chain_hash.clone())
            .unwrap_or_default();
        let entry =
            AuditEntry::new(severity, source, action, details, prev_hash).with_agent(agent_id);
        debug!(
            hash = entry.hash(),
            source = entry.source(),
            action = entry.action(),
            severity = entry.severity().as_str(),
            agent = entry.agent_id().unwrap_or(""),
            index = self.entries.len(),
            "chain entry appended"
        );
        self.entries.push(entry);
        self.entries.last().unwrap()
    }

    /// Number of entries in the chain.
    #[inline]
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    #[inline]
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Get all entries.
    #[inline]
    #[must_use]
    pub fn entries(&self) -> &[AuditEntry] {
        &self.entries
    }

    /// Get the last entry's hash (chain head).
    #[inline]
    #[must_use]
    pub fn head_hash(&self) -> Option<&str> {
        self.entries.last().map(|e| e.hash())
    }

    /// Verify the entire chain's integrity.
    ///
    /// Checks the genesis entry links to the expected previous chain hash
    /// (empty string for a fresh chain, or the archived head after rotation),
    /// then delegates entry-level hash and linkage verification to [`verify_chain`].
    pub fn verify(&self) -> crate::Result<()> {
        if self.entries.is_empty() {
            return Ok(());
        }

        // Verify genesis entry links correctly (accounts for rotation)
        let expected_genesis_prev = self.prev_chain_hash.as_deref().unwrap_or("");
        if self.entries[0].prev_hash() != expected_genesis_prev {
            return Err(LibroError::IntegrityViolation {
                index: 0,
                expected: expected_genesis_prev.to_owned(),
                actual: self.entries[0].prev_hash().to_owned(),
            });
        }

        let result = verify_chain(&self.entries);
        match &result {
            Ok(()) => info!(entries = self.entries.len(), "chain verification passed"),
            Err(e) => warn!(error = %e, "chain verification failed"),
        }
        result
    }

    /// Append multiple events in one call. Each entry is chained to the previous.
    /// Returns a slice of the newly appended entries.
    pub fn append_batch(
        &mut self,
        events: impl IntoIterator<Item = (EventSeverity, String, String, serde_json::Value)>,
    ) -> &[AuditEntry] {
        let start = self.entries.len();
        for (severity, source, action, details) in events {
            self.append(severity, source, action, details);
        }
        &self.entries[start..]
    }

    /// Query entries by source.
    pub fn by_source(&self, source: &str) -> Vec<&AuditEntry> {
        self.entries
            .iter()
            .filter(|e| e.source() == source)
            .collect()
    }

    /// Query entries by severity.
    pub fn by_severity(&self, severity: EventSeverity) -> Vec<&AuditEntry> {
        self.entries
            .iter()
            .filter(|e| e.severity() == severity)
            .collect()
    }

    /// Query entries by agent ID.
    pub fn by_agent(&self, agent_id: &str) -> Vec<&AuditEntry> {
        self.entries
            .iter()
            .filter(|e| e.agent_id() == Some(agent_id))
            .collect()
    }

    /// Return a page of entries: `offset` entries skipped, up to `limit` returned.
    pub fn page(&self, offset: usize, limit: usize) -> &[AuditEntry] {
        let start = offset.min(self.entries.len());
        let end = (start + limit).min(self.entries.len());
        &self.entries[start..end]
    }

    /// Query entries using a composable [`QueryFilter`].
    pub fn query(&self, filter: &QueryFilter) -> Vec<&AuditEntry> {
        filter.apply(&self.entries)
    }

    /// Rotate the chain: drain all current entries and return them as an archive.
    /// The next entry appended will link to the previous chain's head hash,
    /// preserving continuity across rotations.
    pub fn rotate(&mut self) -> ChainArchive {
        let head_hash = self.head_hash().unwrap_or("").to_owned();
        let entries = std::mem::take(&mut self.entries);
        if !head_hash.is_empty() {
            self.prev_chain_hash = Some(head_hash.clone());
        }
        info!(
            archived = entries.len(),
            head_hash = %head_hash,
            "chain rotated"
        );
        ChainArchive { entries, head_hash }
    }

    /// Restore a chain from an archive (e.g. for verification of historical data).
    pub fn from_entries(entries: Vec<AuditEntry>) -> Self {
        Self {
            entries,
            prev_chain_hash: None,
        }
    }

    /// Apply a retention policy, archiving entries that fall outside the
    /// retention window. Returns the archived entries (if any).
    ///
    /// The chain maintains integrity: the first retained entry links to
    /// the last archived entry's hash via `prev_chain_hash`.
    ///
    /// Returns `None` if no entries need archiving.
    pub fn apply_retention(&mut self, policy: &RetentionPolicy) -> Option<ChainArchive> {
        let split = policy.split_index(self.entries());
        if split == 0 {
            return None;
        }

        let mut all_entries = std::mem::take(&mut self.entries);
        let retained = all_entries.split_off(split);

        let head_hash = all_entries
            .last()
            .map(|e| e.hash().to_owned())
            .unwrap_or_default();

        self.entries = retained;
        self.prev_chain_hash = Some(head_hash.clone());

        info!(
            archived = all_entries.len(),
            retained = self.entries.len(),
            "retention policy applied"
        );

        Some(ChainArchive {
            entries: all_entries,
            head_hash,
        })
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
        chain.entries[0].corrupt_action("hacked");
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

    #[test]
    fn rotate_archives_and_continues() {
        let mut chain = AuditChain::new();
        chain.append(EventSeverity::Info, "src", "first", serde_json::json!({}));
        chain.append(EventSeverity::Info, "src", "second", serde_json::json!({}));
        let head_before = chain.head_hash().unwrap().to_owned();

        let archive = chain.rotate();
        assert_eq!(archive.entries.len(), 2);
        assert_eq!(archive.head_hash, head_before);
        assert!(chain.is_empty());

        // New entry links to the archived chain's head
        let entry = chain.append(EventSeverity::Info, "src", "third", serde_json::json!({}));
        assert_eq!(entry.prev_hash(), head_before);
        assert!(chain.verify().is_ok());
    }

    #[test]
    fn rotate_empty_chain() {
        let mut chain = AuditChain::new();
        let archive = chain.rotate();
        assert!(archive.entries.is_empty());
        assert_eq!(archive.head_hash, "");
        // Empty rotation should NOT set prev_chain_hash
        assert!(chain.prev_chain_hash.is_none());
    }

    #[test]
    fn from_entries_verifies() {
        let mut chain = AuditChain::new();
        chain.append(EventSeverity::Info, "s", "a", serde_json::json!({}));
        chain.append(EventSeverity::Info, "s", "b", serde_json::json!({}));
        let entries = chain.entries().to_vec();

        let restored = AuditChain::from_entries(entries);
        assert_eq!(restored.len(), 2);
        assert!(restored.verify().is_ok());
    }

    #[test]
    fn multiple_rotations() {
        let mut chain = AuditChain::new();
        chain.append(EventSeverity::Info, "src", "gen1", serde_json::json!({}));
        let archive1 = chain.rotate();

        chain.append(EventSeverity::Info, "src", "gen2", serde_json::json!({}));
        assert!(chain.verify().is_ok());
        assert_eq!(chain.entries()[0].prev_hash(), archive1.head_hash);

        let archive2 = chain.rotate();
        chain.append(EventSeverity::Info, "src", "gen3", serde_json::json!({}));
        assert!(chain.verify().is_ok());
        assert_eq!(chain.entries()[0].prev_hash(), archive2.head_hash);
    }

    #[test]
    fn by_agent() {
        let mut chain = AuditChain::new();
        chain.append(
            EventSeverity::Info,
            "daimon",
            "start",
            serde_json::json!({}),
        );
        chain.append_with_agent(
            EventSeverity::Info,
            "daimon",
            "task",
            serde_json::json!({}),
            "agent-01",
        );

        assert_eq!(chain.by_agent("agent-01").len(), 1);
        assert_eq!(chain.by_agent("nonexistent").len(), 0);
    }

    #[test]
    fn query_with_filter() {
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

        let results = chain.query(
            &QueryFilter::new()
                .source("daimon")
                .severity(EventSeverity::Info),
        );
        assert_eq!(results.len(), 2);

        let results = chain.query(&QueryFilter::new().action("alert"));
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn append_batch_chains_correctly() {
        let mut chain = AuditChain::new();
        let events = vec![
            (
                EventSeverity::Info,
                "daimon".to_owned(),
                "start".to_owned(),
                serde_json::json!({}),
            ),
            (
                EventSeverity::Security,
                "aegis".to_owned(),
                "alert".to_owned(),
                serde_json::json!({}),
            ),
            (
                EventSeverity::Info,
                "daimon".to_owned(),
                "stop".to_owned(),
                serde_json::json!({}),
            ),
        ];
        let appended = chain.append_batch(events);
        assert_eq!(appended.len(), 3);
        assert_eq!(chain.len(), 3);
        assert!(chain.verify().is_ok());
    }

    #[test]
    fn page_returns_slice() {
        let mut chain = AuditChain::new();
        for i in 0..10 {
            chain.append(
                EventSeverity::Info,
                "s",
                format!("e{i}"),
                serde_json::json!({}),
            );
        }
        assert_eq!(chain.page(0, 3).len(), 3);
        assert_eq!(chain.page(0, 3)[0].action(), "e0");
        assert_eq!(chain.page(3, 3).len(), 3);
        assert_eq!(chain.page(3, 3)[0].action(), "e3");
        assert_eq!(chain.page(8, 5).len(), 2); // only 2 left
        assert_eq!(chain.page(20, 5).len(), 0); // past end
    }

    #[test]
    fn verify_detects_bad_genesis_prev_hash() {
        // Manually construct a chain where genesis has wrong prev_hash
        let entry = AuditEntry::new(
            EventSeverity::Info,
            "s",
            "a",
            serde_json::json!({}),
            "bogus",
        );
        let chain = AuditChain::from_entries(vec![entry]);
        // from_entries sets prev_chain_hash to None, so expected genesis prev is ""
        let err = chain.verify().unwrap_err();
        assert!(err.to_string().contains("entry 0"));
    }

    #[test]
    fn verify_detects_broken_linkage() {
        let e1 = AuditEntry::new(EventSeverity::Info, "s", "a", serde_json::json!({}), "");
        let e2 = AuditEntry::new(
            EventSeverity::Info,
            "s",
            "b",
            serde_json::json!({}),
            "wrong-hash",
        );
        let chain = AuditChain::from_entries(vec![e1, e2]);
        let err = chain.verify().unwrap_err();
        assert!(err.to_string().contains("entry 1"));
    }

    #[test]
    fn verify_detects_tampered_self_hash() {
        let mut chain = AuditChain::new();
        chain.append(EventSeverity::Info, "s", "a", serde_json::json!({}));
        chain.append(EventSeverity::Info, "s", "b", serde_json::json!({}));
        // Tamper with the hash directly (not the content)
        chain.entries[1].corrupt_hash("tampered");
        let err = chain.verify().unwrap_err();
        assert!(err.to_string().contains("entry 1"));
    }

    #[test]
    fn append_with_agent_on_chain() {
        let mut chain = AuditChain::new();
        chain.append(
            EventSeverity::Info,
            "daimon",
            "start",
            serde_json::json!({}),
        );
        let head = chain.head_hash().unwrap().to_owned();
        let entry = chain.append_with_agent(
            EventSeverity::Info,
            "daimon",
            "task",
            serde_json::json!({}),
            "agent-007",
        );
        assert_eq!(entry.agent_id(), Some("agent-007"));
        assert_eq!(entry.prev_hash(), head);
        assert!(entry.verify());
        assert!(chain.verify().is_ok());
    }
}
