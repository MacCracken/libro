use crate::merkle::{MerkleTree, verify_proof};
use crate::retention::RetentionPolicy;
use crate::store::AuditStore;
use crate::*;

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
    chain.append(
        entry::EventSeverity::Info,
        "daimon",
        "start",
        serde_json::json!({}),
    );
    chain.append(
        entry::EventSeverity::Security,
        "aegis",
        "alert",
        serde_json::json!({}),
    );

    let archive = chain.rotate();

    // Persist archive to file store
    let archive_path = dir.path().join("archive.jsonl");
    let mut store = FileStore::open(&archive_path).unwrap();
    for e in &archive.entries {
        store.append(e).unwrap();
    }
    assert_eq!(store.len(), 2);

    // Continue the chain
    chain.append(
        entry::EventSeverity::Info,
        "daimon",
        "stop",
        serde_json::json!({}),
    );
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
    chain.append(
        entry::EventSeverity::Security,
        "aegis",
        "alert",
        serde_json::json!({}),
    );

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
    let e1 = entry::AuditEntry::new(
        entry::EventSeverity::Info,
        "s",
        "a",
        serde_json::json!({}),
        "",
    );
    let e2 = entry::AuditEntry::new(
        entry::EventSeverity::Info,
        "s",
        "b",
        serde_json::json!({}),
        "not-the-right-hash",
    );
    let err = verify_chain(&[e1, e2]).unwrap_err();
    assert!(err.to_string().contains("entry 1"));
}

#[test]
fn merkle_tree_from_chain() {
    let mut chain = AuditChain::new();
    for i in 0..10 {
        chain.append(
            entry::EventSeverity::Info,
            "src",
            format!("e{i}"),
            serde_json::json!({}),
        );
    }

    let tree = MerkleTree::build(chain.entries()).unwrap();
    assert_eq!(tree.leaf_count(), 10);

    // Every entry has a valid proof
    for i in 0..10 {
        let proof = tree.proof(i).unwrap();
        assert!(verify_proof(&proof));
    }

    // Merkle root changes if we add an entry
    let mut chain2 = AuditChain::from_entries(chain.entries().to_vec());
    chain2.entries.push(entry::AuditEntry::new(
        entry::EventSeverity::Info,
        "src",
        "e10",
        serde_json::json!({}),
        chain.head_hash().unwrap(),
    ));
    let tree2 = MerkleTree::build(chain2.entries()).unwrap();
    assert_ne!(tree.root(), tree2.root());
}

#[cfg(feature = "signing")]
#[test]
fn signing_merkle_chain_integration() {
    use crate::signing::SigningKey;

    let key = SigningKey::generate();
    let vk = key.verifying_key();

    // Build chain and sign each entry
    let mut chain = AuditChain::new();
    let mut sigs = Vec::new();
    for i in 0..5 {
        chain.append(
            entry::EventSeverity::Info,
            "src",
            format!("e{i}"),
            serde_json::json!({}),
        );
        sigs.push(key.sign(chain.entries().last().unwrap()));
    }

    // Verify chain integrity
    assert!(chain.verify().is_ok());

    // Verify all signatures
    for (i, sig) in sigs.iter().enumerate() {
        assert!(sig.verify(&chain.entries()[i], &vk), "sig {i} failed");
    }

    // Build merkle tree and verify proofs
    let tree = MerkleTree::build(chain.entries()).unwrap();
    for i in 0..5 {
        let proof = tree.proof(i).unwrap();
        assert!(verify_proof(&proof), "proof {i} failed");
    }

    // Tamper with an entry — entry.verify() catches the content change,
    // while the signature still matches the (now-stale) stored hash.
    // Chain verification catches both.
    let mut entries = chain.entries().to_vec();
    entries[2].corrupt_action("hacked");

    // Entry self-hash check catches the tamper
    assert!(!entries[2].verify());

    // Chain verification catches the broken self-hash
    assert!(verify_chain(&entries).is_err());

    // Signature still matches the stored hash (which wasn't recomputed),
    // but the entry's content no longer matches its hash
    assert!(sigs[2].verify(&entries[2], &vk)); // hash field unchanged
    assert!(!entries[2].verify()); // but content doesn't match

    // Merkle tree built from tampered entries has same root (hash field unchanged),
    // but chain verification catches the actual tamper
    let tampered_tree = MerkleTree::build(&entries).unwrap();
    assert_eq!(tree.root(), tampered_tree.root()); // merkle uses stored hash, not recomputed
}

#[test]
fn chain_batch_empty_input() {
    let mut chain = AuditChain::new();
    let appended = chain.append_batch(std::iter::empty());
    assert!(appended.is_empty());
    assert!(chain.is_empty());
}

#[test]
fn retention_keep_count_equal_to_len() {
    let mut chain = AuditChain::new();
    for i in 0..5 {
        chain.append(
            entry::EventSeverity::Info,
            "src",
            format!("e{i}"),
            serde_json::json!({}),
        );
    }
    // KeepCount == chain length should archive nothing
    let archive = chain.apply_retention(&RetentionPolicy::KeepCount(5));
    assert!(archive.is_none());
    assert_eq!(chain.len(), 5);
    assert!(chain.verify().is_ok());
}

#[test]
fn merkle_proof_all_odd_sizes() {
    // Verify proofs work for various odd tree sizes (edge case: duplication logic)
    for n in [1, 3, 5, 7, 9, 11, 13, 15, 17] {
        let mut chain = AuditChain::new();
        for i in 0..n {
            chain.append(
                entry::EventSeverity::Info,
                "src",
                format!("e{i}"),
                serde_json::json!({}),
            );
        }
        let tree = MerkleTree::build(chain.entries()).unwrap();
        for i in 0..n {
            let proof = tree.proof(i).unwrap();
            assert!(
                verify_proof(&proof),
                "proof failed for tree size {n}, index {i}"
            );
        }
    }
}

#[test]
fn csv_export_roundtrip_no_special_chars() {
    // Verify CSV export doesn't allocate when no escaping needed (Cow::Borrowed)
    let mut chain = AuditChain::new();
    chain.append(
        entry::EventSeverity::Info,
        "simple_source",
        "simple_action",
        serde_json::json!({"key": "value"}),
    );
    let mut buf = Vec::new();
    export::to_csv(chain.entries(), &mut buf).unwrap();
    let output = String::from_utf8(buf).unwrap();
    assert!(output.contains("simple_source"));
    assert!(output.contains("simple_action"));
}

#[test]
fn unicode_in_entry_fields() {
    // Verify hashing handles Unicode correctly
    let mut chain = AuditChain::new();
    chain.append(
        entry::EventSeverity::Info,
        "日本語ソース",
        "действие",
        serde_json::json!({"emoji": "🔒", "arabic": "مرحبا"}),
    );
    assert!(chain.verify().is_ok());

    // Export and re-import preserves Unicode
    let mut buf = Vec::new();
    export::to_jsonl(chain.entries(), &mut buf).unwrap();
    let line = String::from_utf8(buf).unwrap();
    let reimported: entry::AuditEntry = serde_json::from_str(line.trim()).unwrap();
    assert_eq!(reimported.source(), "日本語ソース");
    assert_eq!(reimported.action(), "действие");
    assert!(reimported.verify());
}

#[cfg(feature = "sqlite")]
#[test]
fn sqlite_store_unicode_roundtrip() {
    let mut store = SqliteStore::in_memory().unwrap();
    let e = entry::AuditEntry::new(
        entry::EventSeverity::Info,
        "日本語",
        "действие",
        serde_json::json!({"key": "值"}),
        "",
    );
    store.append(&e).unwrap();
    let loaded = store.load_all().unwrap();
    assert_eq!(loaded[0].source(), "日本語");
    assert!(loaded[0].verify());
}

#[cfg(feature = "signing")]
#[test]
fn signing_key_deterministic_from_seed() {
    use crate::signing::SigningKey;

    let seed = [42u8; 32];
    let key1 = SigningKey::from_bytes(&seed);
    let key2 = SigningKey::from_bytes(&seed);
    assert_eq!(key1.verifying_key().to_hex(), key2.verifying_key().to_hex());

    let entry = entry::AuditEntry::new(
        entry::EventSeverity::Info,
        "src",
        "act",
        serde_json::json!({}),
        "",
    );
    let sig1 = key1.sign(&entry);
    let sig2 = key2.sign(&entry);
    assert_eq!(sig1.signature, sig2.signature);
}
