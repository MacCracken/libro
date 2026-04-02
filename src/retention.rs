//! Retention policies for audit chains.
//!
//! Retention works by rotating the chain: entries that fall outside the
//! retention window are archived (returned as a [`crate::ChainArchive`]), and
//! the chain continues with only the retained entries. This preserves
//! chain integrity — entries are never silently deleted.

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};

use crate::entry::AuditEntry;

/// A retention policy that determines which entries to keep.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum RetentionPolicy {
    /// Keep the most recent N entries.
    KeepCount(usize),
    /// Keep entries newer than this duration.
    KeepDuration(Duration),
    /// Keep entries newer than this absolute timestamp.
    KeepAfter(DateTime<Utc>),
}

/// Custom serialization for `RetentionPolicy`.
///
/// Serializes as a tagged enum with `type` and `value` fields:
/// - `KeepCount` → `{"type": "KeepCount", "value": 1000}`
/// - `KeepDuration` → `{"type": "KeepDuration", "value": 86400}` (seconds)
/// - `KeepAfter` → `{"type": "KeepAfter", "value": "2026-03-01T00:00:00Z"}`
impl Serialize for RetentionPolicy {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("RetentionPolicy", 2)?;
        match self {
            RetentionPolicy::KeepCount(n) => {
                state.serialize_field("type", "KeepCount")?;
                state.serialize_field("value", n)?;
            }
            RetentionPolicy::KeepDuration(d) => {
                state.serialize_field("type", "KeepDuration")?;
                state.serialize_field("value", &d.num_seconds())?;
            }
            RetentionPolicy::KeepAfter(dt) => {
                state.serialize_field("type", "KeepAfter")?;
                state.serialize_field("value", &dt.to_rfc3339())?;
            }
        }
        state.end()
    }
}

impl<'de> Deserialize<'de> for RetentionPolicy {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        struct Tagged {
            r#type: String,
            value: serde_json::Value,
        }
        let tagged = Tagged::deserialize(deserializer)?;
        match tagged.r#type.as_str() {
            "KeepCount" => {
                let n = tagged
                    .value
                    .as_u64()
                    .ok_or_else(|| serde::de::Error::custom("KeepCount value must be a number"))?;
                Ok(RetentionPolicy::KeepCount(n as usize))
            }
            "KeepDuration" => {
                let secs = tagged.value.as_i64().ok_or_else(|| {
                    serde::de::Error::custom("KeepDuration value must be seconds")
                })?;
                Ok(RetentionPolicy::KeepDuration(Duration::seconds(secs)))
            }
            "KeepAfter" => {
                let s = tagged.value.as_str().ok_or_else(|| {
                    serde::de::Error::custom("KeepAfter value must be an RFC3339 timestamp")
                })?;
                let dt = chrono::DateTime::parse_from_rfc3339(s)
                    .map_err(serde::de::Error::custom)?
                    .with_timezone(&Utc);
                Ok(RetentionPolicy::KeepAfter(dt))
            }
            other => Err(serde::de::Error::custom(format!(
                "unknown RetentionPolicy type: {other}"
            ))),
        }
    }
}

impl RetentionPolicy {
    // --- Compliance presets ---

    /// PCI DSS 4.0 Requirement 10.7: retain audit logs for at least 12 months.
    #[must_use]
    pub fn pci_dss() -> Self {
        RetentionPolicy::KeepDuration(Duration::days(365))
    }

    /// HIPAA 45 CFR 164.530(j): retain records for 6 years.
    #[must_use]
    pub fn hipaa() -> Self {
        RetentionPolicy::KeepDuration(Duration::days(6 * 365))
    }

    /// SOX Section 802: retain audit records for 7 years.
    #[must_use]
    pub fn sox() -> Self {
        RetentionPolicy::KeepDuration(Duration::days(7 * 365))
    }

    /// GDPR-aligned: retain for a specified purpose duration.
    /// GDPR does not mandate a fixed period — use the duration appropriate
    /// to your data processing purpose.
    #[must_use]
    pub fn gdpr(purpose_duration: Duration) -> Self {
        RetentionPolicy::KeepDuration(purpose_duration)
    }

    /// Returns the split index: entries before this index are archived,
    /// entries from this index onward are retained.
    pub(crate) fn split_index(&self, entries: &[AuditEntry]) -> usize {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chain::AuditChain;
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
        let archive = chain.apply_retention(&RetentionPolicy::KeepDuration(Duration::hours(1)));
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
        let archive = chain
            .apply_retention(&RetentionPolicy::KeepCount(5))
            .unwrap();

        // Append new entry after retention
        chain.append(EventSeverity::Info, "src", "new", serde_json::json!({}));
        assert!(chain.verify().is_ok());

        // Archive chain should also be independently valid
        let archived_chain = AuditChain::from_entries(archive.entries);
        assert!(archived_chain.verify().is_ok());
    }
}
