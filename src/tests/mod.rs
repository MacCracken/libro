use crate::*;
use crate::store::AuditStore;

#[test]
fn full_chain_lifecycle() {
    let mut chain = AuditChain::new();
    chain.append(
        entry::EventSeverity::Info,
        "daimon",
        "agent.registered",
        serde_json::json!({"agent": "a1"}),
    );
    chain.append(
        entry::EventSeverity::Security,
        "aegis",
        "policy.applied",
        serde_json::json!({"policy": "strict"}),
    );
    chain.append(
        entry::EventSeverity::Info,
        "daimon",
        "agent.deregistered",
        serde_json::json!({"agent": "a1"}),
    );

    assert_eq!(chain.len(), 3);
    assert!(chain.verify().is_ok());
    assert!(verify_chain(chain.entries()).is_ok());
}

#[test]
fn tamper_detection() {
    let mut chain = AuditChain::new();
    chain.append(
        entry::EventSeverity::Info,
        "src",
        "create",
        serde_json::json!({}),
    );
    chain.append(
        entry::EventSeverity::Info,
        "src",
        "update",
        serde_json::json!({}),
    );

    // Verify passes before tamper
    assert!(chain.verify().is_ok());
}

#[test]
fn error_display() {
    let err = LibroError::IntegrityViolation {
        index: 5,
        expected: "abc".into(),
        actual: "xyz".into(),
    };
    assert!(err.to_string().contains("5"));
    assert!(err.to_string().contains("abc"));
}

#[test]
fn chain_rotate_and_persist_to_file_store() {
    let dir = tempfile::tempdir().unwrap();

    // Build a chain and rotate it to a file store
    let mut chain = AuditChain::new();
    chain.append(entry::EventSeverity::Info, "daimon", "start", serde_json::json!({}));
    chain.append(entry::EventSeverity::Security, "aegis", "alert", serde_json::json!({}));

    let archive = chain.rotate();

    // Persist archive to file store
    let archive_path = dir.path().join("archive.jsonl");
    let mut store = FileStore::open(&archive_path).unwrap();
    for e in &archive.entries {
        store.append(e).unwrap();
    }
    assert_eq!(store.len(), 2);

    // Continue the chain
    chain.append(entry::EventSeverity::Info, "daimon", "stop", serde_json::json!({}));
    assert!(chain.verify().is_ok());

    // Reload archive and verify
    let loaded = store.load_all().unwrap();
    assert!(verify_chain(&loaded).is_ok());

    // Verify continuity: new chain's first entry links to archive head
    assert_eq!(chain.entries()[0].prev_hash(), archive.head_hash);
}

#[test]
fn error_variants_display() {
    let store_err = LibroError::Store("connection failed".into());
    assert!(store_err.to_string().contains("connection failed"));

    let io_err = LibroError::Io(std::io::Error::new(std::io::ErrorKind::NotFound, "missing"));
    assert!(io_err.to_string().contains("missing"));

    let json_err: Result<serde_json::Value> = serde_json::from_str("{bad}").map_err(Into::into);
    assert!(json_err.is_err());
}

#[test]
fn chain_from_entries_and_verify_roundtrip() {
    let mut chain = AuditChain::new();
    for i in 0..10 {
        chain.append(
            entry::EventSeverity::Info,
            "src",
            format!("action-{i}"),
            serde_json::json!({"i": i}),
        );
    }
    assert!(chain.verify().is_ok());

    let entries = chain.entries().to_vec();
    let restored = AuditChain::from_entries(entries.clone());
    assert!(restored.verify().is_ok());
    assert!(verify_chain(&entries).is_ok());
}
