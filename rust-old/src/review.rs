//! Chain review — structured summary and per-entry audit trail.
//!
//! Use [`AuditChain::review`] to produce a [`ChainReview`] that summarizes
//! the chain's state, integrity, and contents.

use std::collections::BTreeMap;
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::chain::AuditChain;
use crate::entry::abbreviate_hash;

/// A structured review of an audit chain's contents and integrity.
#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ChainReview {
    /// Total number of entries.
    pub entry_count: usize,
    /// Whether the chain passed integrity verification.
    pub integrity: IntegrityStatus,
    /// Earliest entry timestamp (None if chain is empty).
    pub earliest: Option<String>,
    /// Latest entry timestamp (None if chain is empty).
    pub latest: Option<String>,
    /// Count of entries per source.
    pub sources: BTreeMap<String, usize>,
    /// Count of entries per severity level.
    pub severities: BTreeMap<String, usize>,
    /// Count of entries per agent (None-agent entries counted under "(none)").
    pub agents: BTreeMap<String, usize>,
    /// Head hash of the chain (None if empty).
    pub head_hash: Option<String>,
    /// Whether this chain continues from a previous (rotated) chain.
    pub continued_from: Option<String>,
}

/// Chain integrity status.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum IntegrityStatus {
    /// Chain verified successfully.
    Valid,
    /// Chain is empty.
    Empty,
    /// Chain verification failed.
    Invalid(String),
}

impl fmt::Display for IntegrityStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IntegrityStatus::Valid => write!(f, "VALID"),
            IntegrityStatus::Empty => write!(f, "EMPTY"),
            IntegrityStatus::Invalid(e) => write!(f, "INVALID: {e}"),
        }
    }
}

impl AuditChain {
    /// Produce a structured review of the chain.
    ///
    /// Verifies integrity and summarizes contents: entry count, time range,
    /// source/severity/agent distributions, and head hash.
    pub fn review(&self) -> ChainReview {
        let integrity = if self.is_empty() {
            IntegrityStatus::Empty
        } else {
            match self.verify() {
                Ok(()) => IntegrityStatus::Valid,
                Err(e) => IntegrityStatus::Invalid(e.to_string()),
            }
        };

        let mut sources: BTreeMap<String, usize> = BTreeMap::new();
        let mut severities: BTreeMap<String, usize> = BTreeMap::new();
        let mut agents: BTreeMap<String, usize> = BTreeMap::new();

        for entry in self.entries() {
            *sources.entry(entry.source().to_owned()).or_default() += 1;
            *severities
                .entry(entry.severity().as_str().to_owned())
                .or_default() += 1;
            let agent_key = entry.agent_id().unwrap_or("(none)").to_owned();
            *agents.entry(agent_key).or_default() += 1;
        }

        let earliest = self.entries().first().map(|e| e.timestamp().to_rfc3339());
        let latest = self.entries().last().map(|e| e.timestamp().to_rfc3339());

        ChainReview {
            entry_count: self.len(),
            integrity,
            earliest,
            latest,
            sources,
            severities,
            agents,
            head_hash: self.head_hash().map(|h| h.to_owned()),
            continued_from: self.prev_chain_hash.clone(),
        }
    }
}

impl fmt::Display for ChainReview {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "=== Chain Review ===")?;
        writeln!(f, "Entries:    {}", self.entry_count)?;
        writeln!(f, "Integrity:  {}", self.integrity)?;

        if let Some(ref earliest) = self.earliest {
            writeln!(f, "Earliest:   {earliest}")?;
        }
        if let Some(ref latest) = self.latest {
            writeln!(f, "Latest:     {latest}")?;
        }
        if let Some(ref head) = self.head_hash {
            writeln!(f, "Head:       {}", abbreviate_hash(head))?;
        }
        if let Some(ref prev) = self.continued_from {
            writeln!(f, "Continues:  {}", abbreviate_hash(prev))?;
        }

        if !self.sources.is_empty() {
            writeln!(f, "Sources:")?;
            for (src, count) in &self.sources {
                writeln!(f, "  {src}: {count}")?;
            }
        }
        if !self.severities.is_empty() {
            writeln!(f, "Severities:")?;
            for (sev, count) in &self.severities {
                writeln!(f, "  {sev}: {count}")?;
            }
        }
        if !self.agents.is_empty() {
            writeln!(f, "Agents:")?;
            for (agent, count) in &self.agents {
                writeln!(f, "  {agent}: {count}")?;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::chain::AuditChain;
    use crate::entry::{AuditEntry, EventSeverity};

    use super::*;

    #[test]
    fn review_empty_chain() {
        let chain = AuditChain::new();
        let review = chain.review();
        assert_eq!(review.entry_count, 0);
        assert!(matches!(review.integrity, IntegrityStatus::Empty));
        assert!(review.head_hash.is_none());
        let display = format!("{review}");
        assert!(display.contains("EMPTY"));
    }

    #[test]
    fn review_populated_chain() {
        let mut chain = AuditChain::new();
        chain.append(
            EventSeverity::Info,
            "daimon",
            "agent.start",
            serde_json::json!({}),
        );
        chain.append(
            EventSeverity::Security,
            "aegis",
            "alert",
            serde_json::json!({}),
        );
        chain.append(
            EventSeverity::Info,
            "daimon",
            "agent.stop",
            serde_json::json!({}),
        );

        let review = chain.review();
        assert_eq!(review.entry_count, 3);
        assert!(matches!(review.integrity, IntegrityStatus::Valid));
        assert_eq!(review.sources["daimon"], 2);
        assert_eq!(review.sources["aegis"], 1);
        assert_eq!(review.severities["Info"], 2);
        assert_eq!(review.severities["Security"], 1);
        assert!(review.head_hash.is_some());
        assert!(review.earliest.is_some());
        assert!(review.latest.is_some());

        let display = format!("{review}");
        assert!(display.contains("VALID"));
        assert!(display.contains("daimon: 2"));
        assert!(display.contains("aegis: 1"));
    }

    #[test]
    fn review_tampered_chain() {
        let mut chain = AuditChain::new();
        chain.append(EventSeverity::Info, "src", "act", serde_json::json!({}));
        chain.entries[0].corrupt_action("hacked");

        let review = chain.review();
        assert!(matches!(review.integrity, IntegrityStatus::Invalid(_)));
        let display = format!("{review}");
        assert!(display.contains("INVALID"));
    }

    #[test]
    fn review_with_agents() {
        let mut chain = AuditChain::new();
        chain.append(
            EventSeverity::Info,
            "daimon",
            "start",
            serde_json::json!({}),
        );
        let e = AuditEntry::new(
            EventSeverity::Info,
            "daimon",
            "task",
            serde_json::json!({}),
            chain.head_hash().unwrap(),
        )
        .with_agent("agent-01");
        chain.entries.push(e);

        let review = chain.review();
        assert_eq!(review.agents["agent-01"], 1);
        assert_eq!(review.agents["(none)"], 1);
    }

    #[test]
    fn review_continued_chain() {
        let mut chain = AuditChain::new();
        chain.append(EventSeverity::Info, "src", "act", serde_json::json!({}));
        chain.rotate();
        chain.append(EventSeverity::Info, "src", "act2", serde_json::json!({}));

        let review = chain.review();
        assert!(review.continued_from.is_some());
        let display = format!("{review}");
        assert!(display.contains("Continues:"));
    }

    #[test]
    fn entry_display() {
        let entry = AuditEntry::new(
            EventSeverity::Security,
            "aegis",
            "alert",
            serde_json::json!({}),
            "",
        )
        .with_agent("agent-x");
        let display = format!("{entry}");
        assert!(display.contains("Security"));
        assert!(display.contains("aegis/alert"));
        assert!(display.contains("agent=agent-x"));
    }

    #[test]
    fn severity_display() {
        assert_eq!(format!("{}", EventSeverity::Critical), "Critical");
    }

    #[test]
    fn chain_display_entries() {
        let mut chain = AuditChain::new();
        chain.append(
            EventSeverity::Info,
            "daimon",
            "start",
            serde_json::json!({}),
        );
        chain.append(EventSeverity::Info, "daimon", "stop", serde_json::json!({}));

        // Each entry should be displayable
        for entry in chain.entries() {
            let s = format!("{entry}");
            assert!(s.contains("daimon"));
        }
    }
}
