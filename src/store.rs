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

    /// Verify the chain's integrity without loading all entries into memory at once.
    ///
    /// Reads entries in pages of `chunk_size` and verifies each chunk,
    /// tracking cross-chunk linkage. Returns the total number of verified entries.
    ///
    /// This is O(`chunk_size`) in memory instead of O(N), making it suitable
    /// for stores with millions of entries.
    fn verify_streamed(&self, chunk_size: usize) -> crate::Result<usize> {
        let chunk_size = chunk_size.max(1);
        let mut offset = 0;
        let mut prev_tail_hash: Option<String> = None;
        let mut total = 0;

        loop {
            let chunk = self.load_page(offset, chunk_size)?;
            if chunk.is_empty() {
                break;
            }

            // Verify linkage from previous chunk's last entry
            if let Some(ref expected_prev) = prev_tail_hash
                && chunk[0].prev_hash() != expected_prev
            {
                return Err(crate::LibroError::IntegrityViolation {
                    index: offset,
                    expected: expected_prev.clone(),
                    actual: chunk[0].prev_hash().to_owned(),
                });
            }

            // Verify this chunk internally (adjusting index for error reporting)
            crate::verify::verify_chain_offset(&chunk, offset)?;

            prev_tail_hash = chunk.last().map(|e| e.hash().to_owned());
            total += chunk.len();
            offset += chunk.len();

            if chunk.len() < chunk_size {
                break;
            }
        }

        Ok(total)
    }

    /// Query entries matching a [`QueryFilter`].
    ///
    /// The default implementation loads all entries and filters in memory.
    /// Backends like `SqliteStore` override this with indexed queries.
    fn query(&self, filter: &QueryFilter) -> crate::Result<Vec<AuditEntry>> {
        let all = self.load_all()?;
        Ok(all.into_iter().filter(|e| filter.matches(e)).collect())
    }

    /// Load a page of entries: skip `offset`, return up to `limit`.
    ///
    /// The default implementation loads all entries and slices in memory.
    /// Backends like `SqliteStore` override with SQL LIMIT/OFFSET.
    fn load_page(&self, offset: usize, limit: usize) -> crate::Result<Vec<AuditEntry>> {
        let all = self.load_all()?;
        Ok(all.into_iter().skip(offset).take(limit).collect())
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
        let e2 = AuditEntry::new(
            EventSeverity::Info,
            "s",
            "b",
            serde_json::json!({}),
            e1.hash(),
        );
        store.append(&e1).unwrap();
        store.append(&e2).unwrap();

        let entries = store.load_and_verify().unwrap();
        assert_eq!(entries.len(), 2);
    }

    #[test]
    fn load_and_verify_corrupted() {
        let mut store = MemoryStore::new();
        let e1 = AuditEntry::new(EventSeverity::Info, "s", "a", serde_json::json!({}), "");
        let e2 = AuditEntry::new(
            EventSeverity::Info,
            "s",
            "b",
            serde_json::json!({}),
            "wrong",
        );
        store.append(&e1).unwrap();
        store.append(&e2).unwrap();

        assert!(store.load_and_verify().is_err());
    }

    #[test]
    fn trait_query_default() {
        let mut store = MemoryStore::new();
        let e1 = AuditEntry::new(
            EventSeverity::Info,
            "daimon",
            "start",
            serde_json::json!({}),
            "",
        );
        let e2 = AuditEntry::new(
            EventSeverity::Security,
            "aegis",
            "alert",
            serde_json::json!({}),
            e1.hash(),
        );
        store.append(&e1).unwrap();
        store.append(&e2).unwrap();

        let results = store
            .query(&crate::QueryFilter::new().source("aegis"))
            .unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].source(), "aegis");
    }

    #[test]
    fn load_page_default() {
        let mut store = MemoryStore::new();
        for i in 0..10 {
            let prev = if i == 0 {
                String::new()
            } else {
                store.load_all().unwrap().last().unwrap().hash().to_owned()
            };
            let e = AuditEntry::new(
                EventSeverity::Info,
                "s",
                format!("e{i}"),
                serde_json::json!({}),
                prev,
            );
            store.append(&e).unwrap();
        }
        let page = store.load_page(3, 3).unwrap();
        assert_eq!(page.len(), 3);
        assert_eq!(page[0].action(), "e3");

        let page = store.load_page(8, 5).unwrap();
        assert_eq!(page.len(), 2);
    }

    #[test]
    fn verify_streamed_valid() {
        let mut store = MemoryStore::new();
        let mut prev = String::new();
        for i in 0..20 {
            let e = AuditEntry::new(
                EventSeverity::Info,
                "s",
                format!("e{i}"),
                serde_json::json!({}),
                &prev,
            );
            prev = e.hash().to_owned();
            store.append(&e).unwrap();
        }
        // Verify in chunks of 7 (tests cross-chunk boundary)
        let total = store.verify_streamed(7).unwrap();
        assert_eq!(total, 20);
    }

    #[test]
    fn verify_streamed_corrupted() {
        let mut store = MemoryStore::new();
        let e1 = AuditEntry::new(EventSeverity::Info, "s", "a", serde_json::json!({}), "");
        let e2 = AuditEntry::new(
            EventSeverity::Info,
            "s",
            "b",
            serde_json::json!({}),
            "wrong",
        );
        store.append(&e1).unwrap();
        store.append(&e2).unwrap();

        // Chunk size 1 forces cross-chunk linkage check
        assert!(store.verify_streamed(1).is_err());
    }

    #[test]
    fn verify_streamed_empty() {
        let store = MemoryStore::new();
        let total = store.verify_streamed(10).unwrap();
        assert_eq!(total, 0);
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
