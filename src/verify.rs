//! Chain verification — validate integrity of an audit chain.

use tracing::warn;

use crate::LibroError;
use crate::entry::AuditEntry;

/// Verify a sequence of audit entries forms a valid chain.
pub fn verify_chain(entries: &[AuditEntry]) -> crate::Result<()> {
    if entries.is_empty() {
        return Ok(());
    }

    for (i, entry) in entries.iter().enumerate() {
        if !entry.verify() {
            warn!(
                index = i,
                hash = entry.hash(),
                expected = entry.compute_hash(),
                "entry self-hash verification failed"
            );
            return Err(LibroError::IntegrityViolation {
                index: i,
                expected: entry.compute_hash(),
                actual: entry.hash().to_owned(),
            });
        }
        if i > 0 && entry.prev_hash() != entries[i - 1].hash() {
            warn!(
                index = i,
                expected = entries[i - 1].hash(),
                actual = entry.prev_hash(),
                "chain linkage broken"
            );
            return Err(LibroError::IntegrityViolation {
                index: i,
                expected: entries[i - 1].hash().to_owned(),
                actual: entry.prev_hash().to_owned(),
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
            e1.hash(),
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

    #[test]
    fn verify_tampered_self_hash() {
        let mut e1 =
            AuditEntry::new(EventSeverity::Info, "s", "a", serde_json::json!({}), "");
        e1.corrupt_hash("tampered");
        let err = verify_chain(&[e1]).unwrap_err();
        assert!(err.to_string().contains("entry 0"));
    }

    #[test]
    fn verify_single_valid_entry() {
        let e1 = AuditEntry::new(EventSeverity::Info, "s", "a", serde_json::json!({}), "");
        assert!(verify_chain(&[e1]).is_ok());
    }

    #[test]
    fn verify_long_chain() {
        let mut entries = Vec::new();
        let first = AuditEntry::new(EventSeverity::Info, "s", "e0", serde_json::json!({}), "");
        entries.push(first);
        for i in 1..50 {
            let prev = entries[i - 1].hash();
            entries.push(AuditEntry::new(
                EventSeverity::Info,
                "s",
                format!("e{i}"),
                serde_json::json!({}),
                prev,
            ));
        }
        assert!(verify_chain(&entries).is_ok());

        // Tamper in the middle
        entries[25].corrupt_action("hacked");
        assert!(verify_chain(&entries).is_err());
    }
}
