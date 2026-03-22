//! SQLite-backed audit store for queryable audit history.
//!
//! Stores entries in a single `audit_entries` table with indexed columns
//! for efficient queries by timestamp, severity, source, and agent_id.
//! Requires the `sqlite` feature flag.

use std::sync::Mutex;

use rusqlite::{Connection, params};
use tracing::{debug, info};

use crate::entry::{AuditEntry, EventSeverity};
use crate::query::QueryFilter;
use crate::store::AuditStore;
use crate::LibroError;

const SELECT_COLS: &str =
    "SELECT id, timestamp, severity, source, action, details, agent_id, prev_hash, hash FROM audit_entries";

/// SQLite-backed audit store.
pub struct SqliteStore {
    conn: Mutex<Connection>,
}

impl std::fmt::Debug for SqliteStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SqliteStore").finish_non_exhaustive()
    }
}

impl SqliteStore {
    /// Open or create a SQLite store at the given path.
    pub fn open(path: impl AsRef<std::path::Path>) -> crate::Result<Self> {
        let conn = Connection::open(path).map_err(|e| LibroError::Store(e.to_string()))?;
        let store = Self {
            conn: Mutex::new(conn),
        };
        store.init_schema()?;
        info!("sqlite store opened");
        Ok(store)
    }

    /// Create an in-memory SQLite store (useful for testing).
    pub fn in_memory() -> crate::Result<Self> {
        let conn =
            Connection::open_in_memory().map_err(|e| LibroError::Store(e.to_string()))?;
        let store = Self {
            conn: Mutex::new(conn),
        };
        store.init_schema()?;
        Ok(store)
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Connection> {
        self.conn.lock().expect("sqlite mutex poisoned")
    }

    fn init_schema(&self) -> crate::Result<()> {
        self.lock()
            .execute_batch(
                "CREATE TABLE IF NOT EXISTS audit_entries (
                    seq        INTEGER PRIMARY KEY AUTOINCREMENT,
                    id         TEXT    NOT NULL UNIQUE,
                    timestamp  TEXT    NOT NULL,
                    severity   TEXT    NOT NULL,
                    source     TEXT    NOT NULL,
                    action     TEXT    NOT NULL,
                    details    TEXT    NOT NULL,
                    agent_id   TEXT,
                    prev_hash  TEXT    NOT NULL,
                    hash       TEXT    NOT NULL UNIQUE
                );
                CREATE INDEX IF NOT EXISTS idx_timestamp ON audit_entries(timestamp);
                CREATE INDEX IF NOT EXISTS idx_severity  ON audit_entries(severity);
                CREATE INDEX IF NOT EXISTS idx_source    ON audit_entries(source);
                CREATE INDEX IF NOT EXISTS idx_agent_id  ON audit_entries(agent_id);",
            )
            .map_err(|e| LibroError::Store(e.to_string()))?;
        Ok(())
    }

    fn collect_rows(
        stmt: &mut rusqlite::Statement,
        params: impl rusqlite::Params,
    ) -> crate::Result<Vec<AuditEntry>> {
        let rows = stmt
            .query_map(params, Self::row_to_entry)
            .map_err(|e| LibroError::Store(e.to_string()))?;

        let mut entries = Vec::new();
        for row in rows {
            let entry = row.map_err(|e| LibroError::Store(e.to_string()))?;
            entries.push(entry);
        }
        Ok(entries)
    }

    fn row_to_entry(row: &rusqlite::Row) -> rusqlite::Result<AuditEntry> {
        let id_str: String = row.get(0)?;
        let timestamp_str: String = row.get(1)?;
        let severity_str: String = row.get(2)?;
        let details_str: String = row.get(5)?;
        let agent_id: Option<String> = row.get(6)?;

        let id = uuid::Uuid::parse_str(&id_str).map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(
                0,
                rusqlite::types::Type::Text,
                Box::new(e),
            )
        })?;
        let timestamp = chrono::DateTime::parse_from_rfc3339(&timestamp_str)
            .map_err(|e| {
                rusqlite::Error::FromSqlConversionFailure(
                    1,
                    rusqlite::types::Type::Text,
                    Box::new(e),
                )
            })?
            .with_timezone(&chrono::Utc);
        let severity = match severity_str.as_str() {
            "Debug" => EventSeverity::Debug,
            "Info" => EventSeverity::Info,
            "Warning" => EventSeverity::Warning,
            "Error" => EventSeverity::Error,
            "Critical" => EventSeverity::Critical,
            "Security" => EventSeverity::Security,
            other => {
                return Err(rusqlite::Error::FromSqlConversionFailure(
                    2,
                    rusqlite::types::Type::Text,
                    Box::new(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        format!("unknown severity: {other}"),
                    )),
                ));
            }
        };
        let details: serde_json::Value =
            serde_json::from_str(&details_str).map_err(|e| {
                rusqlite::Error::FromSqlConversionFailure(
                    5,
                    rusqlite::types::Type::Text,
                    Box::new(e),
                )
            })?;

        Ok(AuditEntry::from_raw(
            id,
            timestamp,
            severity,
            row.get(3)?,
            row.get(4)?,
            details,
            agent_id,
            row.get(7)?,
            row.get(8)?,
        ))
    }
}

impl AuditStore for SqliteStore {
    fn append(&mut self, entry: &AuditEntry) -> crate::Result<()> {
        self.lock()
            .execute(
                "INSERT INTO audit_entries (id, timestamp, severity, source, action, details, agent_id, prev_hash, hash)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    entry.id().to_string(),
                    entry.timestamp().to_rfc3339(),
                    entry.severity().as_str(),
                    entry.source(),
                    entry.action(),
                    entry.details().to_string(),
                    entry.agent_id(),
                    entry.prev_hash(),
                    entry.hash(),
                ],
            )
            .map_err(|e| LibroError::Store(e.to_string()))?;
        debug!(hash = entry.hash(), "entry appended to sqlite store");
        Ok(())
    }

    /// Override: translates [`QueryFilter`] to SQL WHERE clauses for indexed queries.
    fn query(&self, filter: &QueryFilter) -> crate::Result<Vec<AuditEntry>> {
        let mut clauses = Vec::new();
        let mut param_values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

        if let Some(ref source) = filter.source {
            param_values.push(Box::new(source.clone()));
            clauses.push(format!("source = ?{}", param_values.len()));
        }
        if let Some(severity) = filter.severity {
            param_values.push(Box::new(severity.as_str().to_owned()));
            clauses.push(format!("severity = ?{}", param_values.len()));
        }
        if let Some(ref agent_id) = filter.agent_id {
            param_values.push(Box::new(agent_id.clone()));
            clauses.push(format!("agent_id = ?{}", param_values.len()));
        }
        if let Some(ref action) = filter.action {
            param_values.push(Box::new(action.clone()));
            clauses.push(format!("action = ?{}", param_values.len()));
        }
        if let Some(after) = filter.after {
            param_values.push(Box::new(after.to_rfc3339()));
            clauses.push(format!("timestamp > ?{}", param_values.len()));
        }
        if let Some(before) = filter.before {
            param_values.push(Box::new(before.to_rfc3339()));
            clauses.push(format!("timestamp < ?{}", param_values.len()));
        }

        let where_clause = if clauses.is_empty() {
            String::new()
        } else {
            format!(" WHERE {}", clauses.join(" AND "))
        };

        let sql = format!("{SELECT_COLS}{where_clause} ORDER BY seq");
        let conn = self.lock();
        let mut stmt = conn
            .prepare(&sql)
            .map_err(|e| LibroError::Store(e.to_string()))?;

        let params: Vec<&dyn rusqlite::types::ToSql> =
            param_values.iter().map(|p| p.as_ref()).collect();
        Self::collect_rows(&mut stmt, params.as_slice())
    }

    fn load_all(&self) -> crate::Result<Vec<AuditEntry>> {
        let conn = self.lock();
        let mut stmt = conn
            .prepare(&format!("{SELECT_COLS} ORDER BY seq"))
            .map_err(|e| LibroError::Store(e.to_string()))?;
        Self::collect_rows(&mut stmt, [])
    }

    fn len(&self) -> usize {
        self.lock()
            .query_row("SELECT COUNT(*) FROM audit_entries", [], |row| row.get(0))
            .unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sqlite_store_roundtrip() {
        let mut store = SqliteStore::in_memory().unwrap();
        assert!(store.is_empty());

        let e1 =
            AuditEntry::new(EventSeverity::Info, "daimon", "agent.start", serde_json::json!({}), "");
        let e2 = AuditEntry::new(
            EventSeverity::Security,
            "aegis",
            "alert",
            serde_json::json!({"ip": "10.0.0.1"}),
            e1.hash(),
        );

        store.append(&e1).unwrap();
        store.append(&e2).unwrap();
        assert_eq!(store.len(), 2);

        let loaded = store.load_all().unwrap();
        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded[0].hash(), e1.hash());
        assert_eq!(loaded[1].hash(), e2.hash());
        assert!(loaded[0].verify());
        assert!(loaded[1].verify());
    }

    #[test]
    fn sqlite_store_query_filter() {
        let mut store = SqliteStore::in_memory().unwrap();
        let e1 = AuditEntry::new(EventSeverity::Info, "daimon", "start", serde_json::json!({}), "")
            .with_agent("agent-01");
        let e2 = AuditEntry::new(EventSeverity::Security, "aegis", "alert", serde_json::json!({}), e1.hash())
            .with_agent("agent-01");
        let e3 = AuditEntry::new(EventSeverity::Info, "daimon", "stop", serde_json::json!({}), e2.hash())
            .with_agent("agent-02");

        store.append(&e1).unwrap();
        store.append(&e2).unwrap();
        store.append(&e3).unwrap();

        // Combined filter
        let results = store.query(&QueryFilter::new().source("daimon").agent_id("agent-01")).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].action(), "start");

        // Action filter
        let results = store.query(&QueryFilter::new().action("alert")).unwrap();
        assert_eq!(results.len(), 1);

        // Empty filter returns all
        let results = store.query(&QueryFilter::new()).unwrap();
        assert_eq!(results.len(), 3);

        // Time range filter
        let after = e1.timestamp();
        let results = store.query(&QueryFilter::new().after(after)).unwrap();
        assert!(results.iter().all(|e| e.timestamp() > after));
    }

    #[test]
    fn sqlite_store_query_severity_and_before() {
        let mut store = SqliteStore::in_memory().unwrap();
        let e1 = AuditEntry::new(EventSeverity::Info, "src", "a", serde_json::json!({}), "");
        let e2 = AuditEntry::new(EventSeverity::Warning, "src", "b", serde_json::json!({}), e1.hash());
        let e3 = AuditEntry::new(EventSeverity::Error, "src", "c", serde_json::json!({}), e2.hash());

        store.append(&e1).unwrap();
        store.append(&e2).unwrap();
        store.append(&e3).unwrap();

        // Severity filter
        let results = store.query(&QueryFilter::new().severity(EventSeverity::Warning)).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].action(), "b");

        // Before filter
        let before = e3.timestamp();
        let results = store.query(&QueryFilter::new().before(before)).unwrap();
        assert!(results.iter().all(|e| e.timestamp() < before));
    }

    #[test]
    fn sqlite_store_load_and_verify() {
        let mut store = SqliteStore::in_memory().unwrap();
        let e1 = AuditEntry::new(EventSeverity::Info, "s", "a", serde_json::json!({}), "");
        let e2 = AuditEntry::new(EventSeverity::Info, "s", "b", serde_json::json!({}), e1.hash());
        store.append(&e1).unwrap();
        store.append(&e2).unwrap();

        let entries = store.load_and_verify().unwrap();
        assert_eq!(entries.len(), 2);
    }

    #[test]
    fn sqlite_store_file_persistence() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("audit.db");

        let e1 = AuditEntry::new(EventSeverity::Info, "src", "act", serde_json::json!({}), "");
        {
            let mut store = SqliteStore::open(&path).unwrap();
            store.append(&e1).unwrap();
        }

        let store = SqliteStore::open(&path).unwrap();
        assert_eq!(store.len(), 1);
        let loaded = store.load_all().unwrap();
        assert_eq!(loaded[0].hash(), e1.hash());
    }
}
