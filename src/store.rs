//! Persistence backends for the audit chain.

use crate::entry::AuditEntry;

/// Trait for audit chain storage backends.
pub trait AuditStore: Send + Sync {
    fn append(&mut self, entry: &AuditEntry) -> crate::Result<()>;
    fn load_all(&self) -> crate::Result<Vec<AuditEntry>>;
    fn len(&self) -> usize;
    fn is_empty(&self) -> bool {
        self.len() == 0
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
