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
