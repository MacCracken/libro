//! Query filters for audit entries.
//!
//! Provides a composable [`QueryFilter`] that works across all backends.
//! In-memory filtering is used for `AuditChain` and `FileStore`;
//! `SqliteStore` translates filters to SQL WHERE clauses.

use chrono::{DateTime, Utc};

use crate::entry::{AuditEntry, EventSeverity};

/// A composable filter for querying audit entries.
///
/// All set fields are ANDed together. Unset fields (None) are ignored.
///
/// ```
/// use libro::query::QueryFilter;
/// use libro::EventSeverity;
///
/// let filter = QueryFilter::new()
///     .source("daimon")
///     .severity(EventSeverity::Security);
/// ```
#[derive(Debug, Clone, Default)]
pub struct QueryFilter {
    pub(crate) source: Option<String>,
    pub(crate) severity: Option<EventSeverity>,
    pub(crate) agent_id: Option<String>,
    pub(crate) after: Option<DateTime<Utc>>,
    pub(crate) before: Option<DateTime<Utc>>,
    pub(crate) action: Option<String>,
}

impl QueryFilter {
    pub fn new() -> Self {
        Self::default()
    }

    /// Filter by source.
    pub fn source(mut self, source: impl Into<String>) -> Self {
        self.source = Some(source.into());
        self
    }

    /// Filter by severity level.
    pub fn severity(mut self, severity: EventSeverity) -> Self {
        self.severity = Some(severity);
        self
    }

    /// Filter by agent ID.
    pub fn agent_id(mut self, agent_id: impl Into<String>) -> Self {
        self.agent_id = Some(agent_id.into());
        self
    }

    /// Filter to entries after this timestamp (exclusive).
    pub fn after(mut self, after: DateTime<Utc>) -> Self {
        self.after = Some(after);
        self
    }

    /// Filter to entries before this timestamp (exclusive).
    pub fn before(mut self, before: DateTime<Utc>) -> Self {
        self.before = Some(before);
        self
    }

    /// Filter by action.
    pub fn action(mut self, action: impl Into<String>) -> Self {
        self.action = Some(action.into());
        self
    }

    /// Test whether a single entry matches this filter.
    pub fn matches(&self, entry: &AuditEntry) -> bool {
        if let Some(ref s) = self.source
            && entry.source() != s
        {
            return false;
        }
        if let Some(sev) = self.severity
            && entry.severity() != sev
        {
            return false;
        }
        if let Some(ref a) = self.agent_id
            && entry.agent_id() != Some(a.as_str())
        {
            return false;
        }
        if let Some(ref a) = self.action
            && entry.action() != a
        {
            return false;
        }
        if let Some(after) = self.after
            && entry.timestamp() <= after
        {
            return false;
        }
        if let Some(before) = self.before
            && entry.timestamp() >= before
        {
            return false;
        }
        true
    }

    /// Filter a slice of entries, returning references to matches.
    pub fn apply<'a>(&self, entries: &'a [AuditEntry]) -> Vec<&'a AuditEntry> {
        entries.iter().filter(|e| self.matches(e)).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_chain() -> Vec<AuditEntry> {
        let e1 = AuditEntry::new(EventSeverity::Info, "daimon", "agent.start", serde_json::json!({}), "")
            .with_agent("agent-01");
        let e2 = AuditEntry::new(EventSeverity::Security, "aegis", "alert", serde_json::json!({}), e1.hash())
            .with_agent("agent-01");
        let e3 = AuditEntry::new(EventSeverity::Info, "daimon", "agent.stop", serde_json::json!({}), e2.hash())
            .with_agent("agent-02");
        vec![e1, e2, e3]
    }

    #[test]
    fn filter_by_source() {
        let entries = make_chain();
        let results = QueryFilter::new().source("daimon").apply(&entries);
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn filter_by_severity() {
        let entries = make_chain();
        let results = QueryFilter::new().severity(EventSeverity::Security).apply(&entries);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].source(), "aegis");
    }

    #[test]
    fn filter_by_agent() {
        let entries = make_chain();
        let results = QueryFilter::new().agent_id("agent-01").apply(&entries);
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn filter_by_action() {
        let entries = make_chain();
        let results = QueryFilter::new().action("alert").apply(&entries);
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn filter_combined() {
        let entries = make_chain();
        let results = QueryFilter::new()
            .source("daimon")
            .agent_id("agent-01")
            .apply(&entries);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].action(), "agent.start");
    }

    #[test]
    fn filter_by_time_range() {
        let entries = make_chain();
        // All entries created in rapid succession — use first entry's timestamp as boundary
        let after = entries[0].timestamp();
        let results = QueryFilter::new().after(after).apply(&entries);
        // Entries after the first (exclusive)
        assert!(results.iter().all(|e| e.timestamp() > after));
    }

    #[test]
    fn filter_no_criteria_matches_all() {
        let entries = make_chain();
        let results = QueryFilter::new().apply(&entries);
        assert_eq!(results.len(), 3);
    }

    #[test]
    fn filter_before_timestamp() {
        let entries = make_chain();
        let last_ts = entries[2].timestamp();
        let results = QueryFilter::new().before(last_ts).apply(&entries);
        assert!(results.iter().all(|e| e.timestamp() < last_ts));
    }

    #[test]
    fn filter_no_matches() {
        let entries = make_chain();
        let results = QueryFilter::new().source("nonexistent").apply(&entries);
        assert!(results.is_empty());
    }
}
