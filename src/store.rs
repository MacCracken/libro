//! Persistence backends for the audit chain.

use crate::entry::AuditEntry;
use crate::query::QueryFilter;

/// Trait for audit chain storage backends.
///
/// **Important:** [`load_all`](AuditStore::load_all) does not verify entry
/// integrity. After loading from an untrusted source, call
/// [`verify_chain`](crate::verify_chain) on the result, or use
/// [`load_and_verify`](AuditStore::load_and_verify).
pub trait AuditStore: Send + Sync {
    fn append(&mut self, entry: &AuditEntry) -> crate::Result<()>;

    /// Load all entries from the store. Does **not** verify integrity.
    fn load_all(&self) -> crate::Result<Vec<AuditEntry>>;

    fn len(&self) -> usize;

    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Load all entries and verify the chain's integrity.
    fn load_and_verify(&self) -> crate::Result<Vec<AuditEntry>> {
        let entries = self.load_all()?;
        crate::verify::verify_chain(&entries)?;
        Ok(entries)
    }

    /// Query entries matching a [`QueryFilter`].
    ///
    /// The default implementation loads all entries and filters in memory.
    /// Backends like `SqliteStore` override this with indexed queries.
    fn query(&self, filter: &QueryFilter) -> crate::Result<Vec<AuditEntry>> {
        let all = self.load_all()?;
        Ok(all.into_iter().filter(|e| filter.matches(e)).collect())
    }
}

/// In-memory store (for testing).
#[derive(Debug, Default)]
pub struct MemoryStore {
    entries: Vec<AuditEntry>,
}

impl MemoryStore {
    pub fn new() -> Self {
        Self::default()
    }
}

impl AuditStore for MemoryStore {
    fn append(&mut self, entry: &AuditEntry) -> crate::Result<()> {
        self.entries.push(entry.clone());
        Ok(())
    }
    fn load_all(&self) -> crate::Result<Vec<AuditEntry>> {
        Ok(self.entries.clone())
    }
    fn len(&self) -> usize {
        self.entries.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entry::{AuditEntry, EventSeverity};

    #[test]
    fn load_and_verify_valid() {
        let mut store = MemoryStore::new();
        let e1 = AuditEntry::new(EventSeverity::Info, "s", "a", serde_json::json!({}), "");
        let e2 = AuditEntry::new(EventSeverity::Info, "s", "b", serde_json::json!({}), e1.hash());
        store.append(&e1).unwrap();
        store.append(&e2).unwrap();

        let entries = store.load_and_verify().unwrap();
        assert_eq!(entries.len(), 2);
    }

    #[test]
    fn load_and_verify_corrupted() {
        let mut store = MemoryStore::new();
        let e1 = AuditEntry::new(EventSeverity::Info, "s", "a", serde_json::json!({}), "");
        let e2 = AuditEntry::new(EventSeverity::Info, "s", "b", serde_json::json!({}), "wrong");
        store.append(&e1).unwrap();
        store.append(&e2).unwrap();

        assert!(store.load_and_verify().is_err());
    }

    #[test]
    fn trait_query_default() {
        let mut store = MemoryStore::new();
        let e1 = AuditEntry::new(EventSeverity::Info, "daimon", "start", serde_json::json!({}), "");
        let e2 = AuditEntry::new(EventSeverity::Security, "aegis", "alert", serde_json::json!({}), e1.hash());
        store.append(&e1).unwrap();
        store.append(&e2).unwrap();

        let results = store.query(&crate::QueryFilter::new().source("aegis")).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].source(), "aegis");
    }

    #[test]
    fn memory_store() {
        let mut store = MemoryStore::new();
        assert!(store.is_empty());
        let entry = AuditEntry::new(EventSeverity::Info, "src", "act", serde_json::json!({}), "");
        store.append(&entry).unwrap();
        assert_eq!(store.len(), 1);
        let loaded = store.load_all().unwrap();
        assert_eq!(loaded[0].hash(), entry.hash());
    }
}
