//! Retention policies for audit chains.
//!
//! Retention works by rotating the chain: entries that fall outside the
//! retention window are archived (returned as a [`ChainArchive`]), and
//! the chain continues with only the retained entries. This preserves
//! chain integrity — entries are never silently deleted.

use chrono::{DateTime, Duration, Utc};

use crate::chain::{AuditChain, ChainArchive};
use crate::entry::AuditEntry;

/// A retention policy that determines which entries to keep.
#[derive(Debug, Clone)]
pub enum RetentionPolicy {
    /// Keep the most recent N entries.
    KeepCount(usize),
    /// Keep entries newer than this duration.
    KeepDuration(Duration),
    /// Keep entries newer than this absolute timestamp.
    KeepAfter(DateTime<Utc>),
}

impl RetentionPolicy {
    /// Returns the split index: entries before this index are archived,
    /// entries from this index onward are retained.
    fn split_index(&self, entries: &[AuditEntry]) -> usize {
        match self {
            RetentionPolicy::KeepCount(n) => entries.len().saturating_sub(*n),
            RetentionPolicy::KeepDuration(duration) => {
                let cutoff = Utc::now() - *duration;
                Self::first_after(entries, cutoff)
            }
            RetentionPolicy::KeepAfter(cutoff) => Self::first_after(entries, *cutoff),
        }
    }

    fn first_after(entries: &[AuditEntry], cutoff: DateTime<Utc>) -> usize {
        entries
            .iter()
            .position(|e| e.timestamp() > cutoff)
            .unwrap_or(entries.len())
    }
}

impl AuditChain {
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

        let all_entries = std::mem::take(&mut self.entries);
        let (archived, retained) = all_entries.split_at(split);

        let head_hash = archived
            .last()
            .map(|e| e.hash().to_owned())
            .unwrap_or_default();

        self.entries = retained.to_vec();
        self.prev_chain_hash = Some(head_hash.clone());

        Some(ChainArchive {
            entries: archived.to_vec(),
            head_hash,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entry::EventSeverity;

    fn build_chain(n: usize) -> AuditChain {
        let mut chain = AuditChain::new();
        for i in 0..n {
            chain.append(
                EventSeverity::Info,
                "src",
                format!("action-{i}"),
                serde_json::json!({}),
            );
        }
        chain
    }

    #[test]
    fn keep_count_retains_last_n() {
        let mut chain = build_chain(10);
        let archive = chain.apply_retention(&RetentionPolicy::KeepCount(3));
        let archive = archive.unwrap();

        assert_eq!(archive.entries.len(), 7);
        assert_eq!(chain.len(), 3);
        assert!(chain.verify().is_ok());
        // First retained entry links to last archived
        assert_eq!(chain.entries()[0].prev_hash(), archive.head_hash);
    }

    #[test]
    fn keep_count_larger_than_chain() {
        let mut chain = build_chain(5);
        let archive = chain.apply_retention(&RetentionPolicy::KeepCount(10));
        assert!(archive.is_none());
        assert_eq!(chain.len(), 5);
    }

    #[test]
    fn keep_count_zero_archives_all() {
        let mut chain = build_chain(5);
        let archive = chain.apply_retention(&RetentionPolicy::KeepCount(0));
        let archive = archive.unwrap();
        assert_eq!(archive.entries.len(), 5);
        assert!(chain.is_empty());
    }

    #[test]
    fn keep_after_timestamp() {
        let mut chain = build_chain(5);
        // Keep entries after the timestamp of entry[2]
        let cutoff = chain.entries()[2].timestamp();
        let archive = chain.apply_retention(&RetentionPolicy::KeepAfter(cutoff));
        let archive = archive.unwrap();

        // Entries 0, 1, 2 have timestamp <= cutoff, so they're archived
        // Entries 3, 4 have timestamp > cutoff (or equal, since created right after)
        assert!(!archive.entries.is_empty());
        assert!(chain.verify().is_ok());
        // All retained entries should be after the cutoff
        for e in chain.entries() {
            assert!(e.timestamp() > cutoff);
        }
    }

    #[test]
    fn keep_duration_recent() {
        let mut chain = build_chain(5);
        // All entries were just created, so keeping 1 hour should retain all
        let archive = chain.apply_retention(&RetentionPolicy::KeepDuration(
            Duration::hours(1),
        ));
        assert!(archive.is_none());
        assert_eq!(chain.len(), 5);
    }

    #[test]
    fn retention_on_empty_chain() {
        let mut chain = AuditChain::new();
        let archive = chain.apply_retention(&RetentionPolicy::KeepCount(5));
        assert!(archive.is_none());
    }

    #[test]
    fn retention_preserves_chain_continuity() {
        let mut chain = build_chain(10);
        let archive = chain.apply_retention(&RetentionPolicy::KeepCount(5)).unwrap();

        // Append new entry after retention
        chain.append(EventSeverity::Info, "src", "new", serde_json::json!({}));
        assert!(chain.verify().is_ok());

        // Archive chain should also be independently valid
        let archived_chain = AuditChain::from_entries(archive.entries);
        assert!(archived_chain.verify().is_ok());
    }
}
