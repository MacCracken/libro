//! Chain verification — validate integrity of an audit chain.

use crate::LibroError;
use crate::entry::AuditEntry;

/// Verify a sequence of audit entries forms a valid chain.
pub fn verify_chain(entries: &[AuditEntry]) -> crate::Result<()> {
    if entries.is_empty() {
        return Ok(());
    }

    for (i, entry) in entries.iter().enumerate() {
        if !entry.verify() {
            return Err(LibroError::IntegrityViolation {
                index: i,
                expected: entry.compute_hash(),
                actual: entry.hash.clone(),
            });
        }
        if i > 0 && entry.prev_hash != entries[i - 1].hash {
            return Err(LibroError::IntegrityViolation {
                index: i,
                expected: entries[i - 1].hash.clone(),
                actual: entry.prev_hash.clone(),
            });
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entry::{AuditEntry, EventSeverity};

    #[test]
    fn verify_valid_chain() {
        let e1 = AuditEntry::new(EventSeverity::Info, "s", "a", serde_json::json!({}), "");
        let e2 = AuditEntry::new(
            EventSeverity::Info,
            "s",
            "b",
            serde_json::json!({}),
            &e1.hash,
        );
        assert!(verify_chain(&[e1, e2]).is_ok());
    }

    #[test]
    fn verify_broken_link() {
        let e1 = AuditEntry::new(EventSeverity::Info, "s", "a", serde_json::json!({}), "");
        let e2 = AuditEntry::new(
            EventSeverity::Info,
            "s",
            "b",
            serde_json::json!({}),
            "wrong",
        );
        assert!(verify_chain(&[e1, e2]).is_err());
    }

    #[test]
    fn verify_empty() {
        assert!(verify_chain(&[]).is_ok());
    }
}
