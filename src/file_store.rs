//! File-based audit store — append-only JSON Lines format.
//!
//! Each entry is written as a single JSON line. Writes acquire an exclusive
//! advisory file lock (`flock`) to prevent interleaved writes from concurrent
//! processes. Reads acquire a shared lock for consistency.

use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

use fs2::FileExt;
use tracing::{debug, error, info};

use crate::entry::AuditEntry;
use crate::query::QueryFilter;
use crate::store::AuditStore;
use crate::LibroError;

/// Append-only file store using JSON Lines format.
#[derive(Debug)]
pub struct FileStore {
    path: PathBuf,
    count: usize,
}

impl FileStore {
    /// Open or create a file store at the given path.
    pub fn open(path: impl AsRef<Path>) -> crate::Result<Self> {
        let path = path.as_ref().to_path_buf();

        // Create the file if it doesn't exist
        if !path.exists() {
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            File::create(&path)?;
        }

        let count = Self::count_lines(&path)?;
        info!(path = %path.display(), entries = count, "file store opened");
        Ok(Self { path, count })
    }

    /// Return the file path.
    pub fn path(&self) -> &Path {
        &self.path
    }

    fn count_lines(path: &Path) -> crate::Result<usize> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let mut count = 0;
        for line in reader.lines() {
            let line = line?;
            if !line.trim().is_empty() {
                count += 1;
            }
        }
        Ok(count)
    }
}

impl FileStore {
    /// Query entries using a [`QueryFilter`].
    ///
    /// Loads all entries from disk and filters in memory.
    pub fn query(&self, filter: &QueryFilter) -> crate::Result<Vec<AuditEntry>> {
        let all = self.load_all()?;
        Ok(all.into_iter().filter(|e| filter.matches(e)).collect())
    }
}

impl AuditStore for FileStore {
    fn append(&mut self, entry: &AuditEntry) -> crate::Result<()> {
        let mut file = OpenOptions::new().append(true).open(&self.path)?;
        file.lock_exclusive()?;
        let json = serde_json::to_string(entry)?;
        writeln!(file, "{json}")?;
        file.unlock()?;
        self.count += 1;
        debug!(hash = entry.hash(), index = self.count - 1, "entry appended to file store");
        Ok(())
    }

    fn load_all(&self) -> crate::Result<Vec<AuditEntry>> {
        let file = File::open(&self.path)?;
        file.lock_shared()?;
        let reader = BufReader::new(file);
        let mut entries = Vec::new();

        for (line_num, line) in reader.lines().enumerate() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            let entry: AuditEntry = serde_json::from_str(&line).map_err(|e| {
                error!(line = line_num + 1, error = %e, "failed to parse entry from file store");
                LibroError::Store(format!("line {}: {e}", line_num + 1))
            })?;
            entries.push(entry);
        }

        Ok(entries)
    }

    fn len(&self) -> usize {
        self.count
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entry::EventSeverity;

    #[test]
    fn file_store_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("audit.jsonl");
        let mut store = FileStore::open(&path).unwrap();

        assert!(store.is_empty());

        let e1 = AuditEntry::new(EventSeverity::Info, "daimon", "agent.start", serde_json::json!({}), "");
        let e2 = AuditEntry::new(EventSeverity::Security, "aegis", "alert", serde_json::json!({"ip": "10.0.0.1"}), e1.hash());

        store.append(&e1).unwrap();
        store.append(&e2).unwrap();
        assert_eq!(store.len(), 2);

        let loaded = store.load_all().unwrap();
        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded[0].hash(), e1.hash());
        assert_eq!(loaded[1].hash(), e2.hash());
        assert_eq!(loaded[1].prev_hash(), e1.hash());
    }

    #[test]
    fn file_store_persistence() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("audit.jsonl");

        // Write with one instance
        {
            let mut store = FileStore::open(&path).unwrap();
            let entry = AuditEntry::new(EventSeverity::Info, "src", "act", serde_json::json!({}), "");
            store.append(&entry).unwrap();
        }

        // Read with a new instance
        let store = FileStore::open(&path).unwrap();
        assert_eq!(store.len(), 1);
        let loaded = store.load_all().unwrap();
        assert_eq!(loaded.len(), 1);
        assert!(loaded[0].verify());
    }

    #[test]
    fn file_store_creates_parent_dirs() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nested").join("dir").join("audit.jsonl");
        let store = FileStore::open(&path).unwrap();
        assert!(store.is_empty());
        assert!(path.exists());
    }

    #[test]
    fn file_store_path_accessor() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("audit.jsonl");
        let store = FileStore::open(&path).unwrap();
        assert_eq!(store.path(), path);
    }

    #[test]
    fn file_store_malformed_line() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("audit.jsonl");
        // Write garbage
        std::fs::write(&path, "not valid json\n").unwrap();
        let store = FileStore::open(&path).unwrap();
        assert_eq!(store.len(), 1); // counts the line
        let err = store.load_all().unwrap_err();
        assert!(err.to_string().contains("line 1"));
    }

    #[test]
    fn file_store_skips_blank_lines() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("audit.jsonl");
        let entry = AuditEntry::new(EventSeverity::Info, "src", "act", serde_json::json!({}), "");
        let json = serde_json::to_string(&entry).unwrap();
        // Write with extra blank lines
        std::fs::write(&path, format!("{json}\n\n{json}\n")).unwrap();
        let store = FileStore::open(&path).unwrap();
        assert_eq!(store.len(), 2); // only non-blank lines
        let loaded = store.load_all().unwrap();
        assert_eq!(loaded.len(), 2);
    }

    #[test]
    fn file_store_query() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("audit.jsonl");
        let mut store = FileStore::open(&path).unwrap();

        let e1 = AuditEntry::new(EventSeverity::Info, "daimon", "start", serde_json::json!({}), "");
        let e2 = AuditEntry::new(EventSeverity::Security, "aegis", "alert", serde_json::json!({}), e1.hash());
        store.append(&e1).unwrap();
        store.append(&e2).unwrap();

        let results = store.query(&crate::query::QueryFilter::new().source("aegis")).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].action(), "alert");
    }

    #[test]
    fn file_store_append_preserves_existing() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("audit.jsonl");

        let e1 = AuditEntry::new(EventSeverity::Info, "src", "first", serde_json::json!({}), "");
        {
            let mut store = FileStore::open(&path).unwrap();
            store.append(&e1).unwrap();
        }

        // Reopen, append more
        let mut store = FileStore::open(&path).unwrap();
        assert_eq!(store.len(), 1);
        let e2 = AuditEntry::new(EventSeverity::Warning, "src", "second", serde_json::json!({}), e1.hash());
        store.append(&e2).unwrap();
        assert_eq!(store.len(), 2);

        let loaded = store.load_all().unwrap();
        assert_eq!(loaded[0].action(), "first");
        assert_eq!(loaded[1].action(), "second");
    }
}
