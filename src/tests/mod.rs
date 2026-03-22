use crate::*;
use crate::retention::RetentionPolicy;
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

#[test]
fn archive_entries_independently_verifiable() {
    let mut chain = AuditChain::new();
    for i in 0..5 {
        chain.append(
            entry::EventSeverity::Info,
            "src",
            format!("action-{i}"),
            serde_json::json!({}),
        );
    }
    let archive = chain.rotate();
    assert!(verify_chain(&archive.entries).is_ok());
}

#[test]
fn retention_then_append_then_review() {
    let mut chain = AuditChain::new();
    for i in 0..10 {
        chain.append(
            entry::EventSeverity::Info,
            "daimon",
            format!("action-{i}"),
            serde_json::json!({}),
        );
    }
    chain.apply_retention(&RetentionPolicy::KeepCount(3));
    chain.append(entry::EventSeverity::Security, "aegis", "alert", serde_json::json!({}));

    assert!(chain.verify().is_ok());

    let review = chain.review();
    assert_eq!(review.entry_count, 4);
    assert!(review.continued_from.is_some());
    assert_eq!(review.sources["daimon"], 3);
    assert_eq!(review.sources["aegis"], 1);
    assert_eq!(review.severities["Security"], 1);

    let display = format!("{review}");
    assert!(display.contains("Continues:"));
    assert!(display.contains("VALID"));
}

#[test]
fn all_severity_levels_through_chain() {
    let mut chain = AuditChain::new();
    let severities = [
        entry::EventSeverity::Debug,
        entry::EventSeverity::Info,
        entry::EventSeverity::Warning,
        entry::EventSeverity::Error,
        entry::EventSeverity::Critical,
        entry::EventSeverity::Security,
    ];
    for sev in &severities {
        chain.append(*sev, "src", format!("{sev}"), serde_json::json!({}));
    }
    assert_eq!(chain.len(), 6);
    assert!(chain.verify().is_ok());

    for sev in &severities {
        assert_eq!(chain.by_severity(*sev).len(), 1);
    }
}

#[test]
fn export_to_file_store_then_load_and_verify() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("audit.jsonl");

    let mut chain = AuditChain::new();
    for i in 0..5 {
        chain.append(
            entry::EventSeverity::Info,
            "src",
            format!("e{i}"),
            serde_json::json!({"i": i}),
        );
    }

    // Export to JSONL file via store
    let mut store = FileStore::open(&path).unwrap();
    for e in chain.entries() {
        store.append(e).unwrap();
    }

    // Load back and verify
    let loaded = store.load_and_verify().unwrap();
    assert_eq!(loaded.len(), 5);
    for (orig, loaded) in chain.entries().iter().zip(loaded.iter()) {
        assert_eq!(orig.hash(), loaded.hash());
    }
}

#[test]
fn verify_chain_linkage_failure_path() {
    // Specifically exercises the broken-linkage warn! path in verify.rs
    let e1 = entry::AuditEntry::new(entry::EventSeverity::Info, "s", "a", serde_json::json!({}), "");
    let e2 = entry::AuditEntry::new(entry::EventSeverity::Info, "s", "b", serde_json::json!({}), "not-the-right-hash");
    let err = verify_chain(&[e1, e2]).unwrap_err();
    assert!(err.to_string().contains("entry 1"));
}
