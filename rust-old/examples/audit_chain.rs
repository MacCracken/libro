//! Basic audit chain example — append, verify, query, export.
//!
//! Run with: `cargo run --example audit_chain`

use libro::{AuditChain, EventSeverity, QueryFilter, RetentionPolicy};

fn main() {
    // Build a chain
    let mut chain = AuditChain::new();

    chain.append(
        EventSeverity::Info,
        "daimon",
        "agent.register",
        serde_json::json!({ "agent_id": "web-agent-01", "sandbox": "landlock" }),
    );

    chain.append(
        EventSeverity::Security,
        "aegis",
        "intrusion.detected",
        serde_json::json!({ "source": "10.0.0.5", "port": 22, "attempts": 5 }),
    );

    chain.append(
        EventSeverity::Info,
        "daimon",
        "agent.deregister",
        serde_json::json!({ "agent_id": "web-agent-01" }),
    );

    // Verify integrity
    chain.verify().expect("chain integrity verified");
    println!("Chain verified: {} entries", chain.len());

    // Query
    let security = chain.query(&QueryFilter::new().min_severity(EventSeverity::Security));
    println!("Security events: {}", security.len());

    let daimon = chain.by_source("daimon");
    println!("Daimon events: {}", daimon.len());

    // Display entries
    for entry in chain.entries() {
        println!("  {entry}");
    }

    // Review
    let review = chain.review();
    print!("{review}");

    // Export to CSV
    let mut csv_buf = Vec::new();
    libro::to_csv(chain.entries(), &mut csv_buf).unwrap();
    println!("CSV export: {} bytes", csv_buf.len());

    // Retention
    let archive = chain
        .apply_retention(&RetentionPolicy::KeepCount(2))
        .expect("should archive");
    println!(
        "Retained {} entries, archived {}",
        chain.len(),
        archive.entries.len()
    );
    chain.verify().expect("chain still valid after retention");

    // Merkle tree
    let tree = libro::MerkleTree::build(chain.entries()).unwrap();
    println!("Merkle root: {}", tree.root());
    let proof = tree.proof(0).unwrap();
    assert!(libro::merkle::verify_proof(&proof));
    println!("Merkle proof for entry 0: valid");
}
