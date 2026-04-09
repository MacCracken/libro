//! Merkle root anchoring to external witness systems.
//!
//! Periodically snapshot a chain's Merkle root and publish it to an external
//! witness system for third-party proof of chain state at a point in time.
//!
//! Libro provides the anchor types and verification logic; consumers implement
//! [`WitnessBackend`] for their specific witness system (transparency log,
//! blockchain, RFC 3161 TSA, file store, etc.).
//!
//! Requires the `anchoring` feature flag.
//!
//! # Usage
//!
//! ```rust,ignore
//! use libro::anchoring::{WitnessAnchor, AnchorVerification};
//! use libro::{AuditChain, MerkleTree, EventSeverity};
//!
//! let mut chain = AuditChain::new();
//! chain.append(EventSeverity::Info, "src", "act", serde_json::json!({}));
//!
//! let tree = MerkleTree::build(chain.entries()).unwrap();
//! let anchor = WitnessAnchor::new(&tree, &chain).unwrap();
//! assert!(anchor.verify_integrity());
//! assert!(anchor.verify_against(&tree, &chain).is_valid());
//! ```

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::chain::AuditChain;
use crate::entry::{constant_time_eq, hash_field};
use crate::hasher::{ChainHasher, HASH_ALGORITHM};
use crate::merkle::MerkleTree;

/// A snapshot of a chain's Merkle root, ready to be anchored to a witness.
///
/// Contains all metadata needed to verify the chain state at the time
/// of anchoring, independent of the witness system used. The anchor is
/// self-hashed for integrity checking.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct WitnessAnchor {
    /// Unique identifier for this anchor.
    pub id: Uuid,
    /// The Merkle root hash being anchored (hex-encoded).
    pub merkle_root: String,
    /// Number of entries covered by this Merkle tree.
    pub entry_count: usize,
    /// The chain head hash at the time of anchoring (hex-encoded).
    pub chain_head: String,
    /// Hash algorithm used (e.g., "blake3" or "sha256").
    pub hash_algorithm: String,
    /// When the anchor was created.
    pub created_at: DateTime<Utc>,
    /// Hash of the previous anchor for anchor chaining (meta-chain).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prev_anchor_hash: Option<String>,
    /// This anchor's self-hash (covers all fields above).
    pub hash: String,
}

/// Receipt from a witness backend after publishing an anchor.
///
/// The receipt proves the anchor was accepted by the witness system.
/// Its contents are backend-specific (e.g., a transaction hash for
/// a blockchain, a signed receipt for a transparency log, or an
/// RFC 3161 token for a TSA).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct WitnessReceipt {
    /// The anchor ID this receipt is for.
    pub anchor_id: Uuid,
    /// The witness backend identifier (e.g., "rfc3161", "transparency-log", "file").
    pub backend: String,
    /// When the receipt was obtained.
    pub received_at: DateTime<Utc>,
    /// Backend-specific receipt data (opaque to libro).
    ///
    /// Examples:
    /// - Blockchain: `{"tx_hash": "0x...", "block": 12345}`
    /// - TSA: `{"token_der": "3082..."}`
    /// - File: `{"path": "/var/anchors/2024-01-15.json"}`
    pub receipt_data: serde_json::Value,
    /// Optional integrity hash of the receipt data.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub receipt_hash: Option<String>,
}

/// Result of verifying an anchor against a chain or Merkle tree.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum AnchorVerification {
    /// The chain matches the anchor exactly.
    Valid,
    /// The Merkle root does not match.
    RootMismatch { expected: String, actual: String },
    /// The entry count does not match.
    CountMismatch { expected: usize, actual: usize },
    /// The chain head does not match.
    HeadMismatch { expected: String, actual: String },
}

/// Trait for witness backends that can store Merkle root anchors.
///
/// Consumers implement this for their witness system. Libro provides
/// the anchor creation and verification; the backend handles persistence
/// and third-party attestation.
pub trait WitnessBackend: Send + Sync {
    /// Publish an anchor to the witness system.
    ///
    /// Returns a receipt proving the anchor was accepted.
    fn publish(&self, anchor: &WitnessAnchor) -> crate::Result<WitnessReceipt>;

    /// Verify that a receipt is still valid for the given anchor.
    ///
    /// Checks backend-specific validity (e.g., blockchain confirmation,
    /// TSA token signature). Returns `true` if valid.
    fn verify_receipt(
        &self,
        anchor: &WitnessAnchor,
        receipt: &WitnessReceipt,
    ) -> crate::Result<bool>;

    /// A human-readable identifier for this backend (e.g., "rfc3161", "ethereum").
    fn backend_id(&self) -> &str;
}

impl WitnessAnchor {
    /// Create an anchor from a Merkle tree and chain.
    ///
    /// Captures the Merkle root, entry count, and chain head at the
    /// current point in time. Returns `None` if the chain is empty.
    #[must_use]
    pub fn new(tree: &MerkleTree, chain: &AuditChain) -> Option<Self> {
        let chain_head = chain.head_hash()?.to_owned();
        Some(Self::from_tree(tree, chain_head))
    }

    /// Create an anchor from a Merkle tree and explicit chain head.
    ///
    /// Useful when the chain is not directly available (e.g., working
    /// from a [`ChainArchive`](crate::ChainArchive) or store-loaded entries).
    #[must_use]
    pub fn from_tree(tree: &MerkleTree, chain_head: impl Into<String>) -> Self {
        let mut anchor = Self {
            id: Uuid::new_v4(),
            merkle_root: tree.root().to_owned(),
            entry_count: tree.leaf_count(),
            chain_head: chain_head.into(),
            hash_algorithm: HASH_ALGORITHM.to_owned(),
            created_at: Utc::now(),
            prev_anchor_hash: None,
            hash: String::new(),
        };
        anchor.hash = anchor.compute_hash();
        anchor
    }

    /// Chain this anchor to a previous anchor, creating a hash-linked
    /// sequence of anchors (meta-chain). Recomputes the self-hash.
    #[must_use]
    pub fn with_prev_anchor(mut self, prev: &WitnessAnchor) -> Self {
        self.prev_anchor_hash = Some(prev.hash.clone());
        self.hash = self.compute_hash();
        self
    }

    /// Verify that this anchor matches the given Merkle tree and chain.
    #[must_use]
    pub fn verify_against(&self, tree: &MerkleTree, chain: &AuditChain) -> AnchorVerification {
        let actual_head = chain.head_hash().unwrap_or("");
        if !constant_time_eq(&self.chain_head, actual_head) {
            return AnchorVerification::HeadMismatch {
                expected: self.chain_head.clone(),
                actual: actual_head.to_owned(),
            };
        }
        self.verify_against_tree(tree)
    }

    /// Verify that this anchor matches the given Merkle tree.
    ///
    /// Checks root hash and entry count only (no chain head check).
    #[must_use]
    pub fn verify_against_tree(&self, tree: &MerkleTree) -> AnchorVerification {
        if self.entry_count != tree.leaf_count() {
            return AnchorVerification::CountMismatch {
                expected: self.entry_count,
                actual: tree.leaf_count(),
            };
        }
        if !constant_time_eq(&self.merkle_root, tree.root()) {
            return AnchorVerification::RootMismatch {
                expected: self.merkle_root.clone(),
                actual: tree.root().to_owned(),
            };
        }
        AnchorVerification::Valid
    }

    /// Verify the anchor's self-hash integrity.
    #[must_use]
    pub fn verify_integrity(&self) -> bool {
        constant_time_eq(&self.hash, &self.compute_hash())
    }

    fn compute_hash(&self) -> String {
        let mut hasher = ChainHasher::new();
        // Fixed-length field (no prefix needed)
        hasher.update(self.id.as_bytes());
        // Variable-length fields: length-prefixed to prevent boundary ambiguity
        hash_field(&mut hasher, self.merkle_root.as_bytes());
        hasher.update(&self.entry_count.to_le_bytes());
        hash_field(&mut hasher, self.chain_head.as_bytes());
        hash_field(&mut hasher, self.hash_algorithm.as_bytes());
        hash_field(&mut hasher, self.created_at.to_rfc3339().as_bytes());
        hash_field(
            &mut hasher,
            self.prev_anchor_hash.as_deref().unwrap_or("").as_bytes(),
        );
        hasher.finalize_hex()
    }
}

impl WitnessReceipt {
    /// Create a receipt manually (for custom backends).
    pub fn new(
        anchor_id: Uuid,
        backend: impl Into<String>,
        receipt_data: serde_json::Value,
    ) -> Self {
        Self {
            anchor_id,
            backend: backend.into(),
            received_at: Utc::now(),
            receipt_data,
            receipt_hash: None,
        }
    }

    /// Create a receipt with an integrity hash over the receipt data.
    ///
    /// The hash uses length-prefixed fields and canonical JSON (sorted keys)
    /// for deterministic hashing regardless of key insertion order.
    pub fn with_hash(mut self) -> Self {
        let mut hasher = ChainHasher::new();
        hasher.update(self.anchor_id.as_bytes());
        hash_field(&mut hasher, self.backend.as_bytes());
        // Use serde_json::to_string for canonical representation
        // (serde_json preserves insertion order, which is deterministic
        // for receipts created from the same source)
        hash_field(
            &mut hasher,
            serde_json::to_string(&self.receipt_data)
                .unwrap_or_default()
                .as_bytes(),
        );
        self.receipt_hash = Some(hasher.finalize_hex());
        self
    }
}

impl AnchorVerification {
    /// Whether the verification passed.
    #[inline]
    #[must_use]
    pub fn is_valid(&self) -> bool {
        matches!(self, Self::Valid)
    }
}

impl std::fmt::Display for AnchorVerification {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Valid => write!(f, "valid"),
            Self::RootMismatch { expected, actual } => {
                write!(f, "root mismatch: expected {expected}, got {actual}")
            }
            Self::CountMismatch { expected, actual } => {
                write!(f, "count mismatch: expected {expected}, got {actual}")
            }
            Self::HeadMismatch { expected, actual } => {
                write!(f, "head mismatch: expected {expected}, got {actual}")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entry::EventSeverity;

    fn make_chain(n: usize) -> AuditChain {
        let mut chain = AuditChain::new();
        for i in 0..n {
            chain.append(
                EventSeverity::Info,
                "s",
                format!("e{i}"),
                serde_json::json!({}),
            );
        }
        chain
    }

    #[test]
    fn anchor_from_chain() {
        let chain = make_chain(5);
        let tree = MerkleTree::build(chain.entries()).unwrap();
        let anchor = WitnessAnchor::new(&tree, &chain).unwrap();

        assert_eq!(anchor.merkle_root, tree.root());
        assert_eq!(anchor.entry_count, 5);
        assert_eq!(anchor.chain_head, chain.head_hash().unwrap());
        assert!(!anchor.hash.is_empty());
        assert!(anchor.prev_anchor_hash.is_none());
    }

    #[test]
    fn anchor_none_for_empty_chain() {
        let chain = AuditChain::new();
        assert!(MerkleTree::build(chain.entries()).is_none());
    }

    #[test]
    fn anchor_from_tree() {
        let chain = make_chain(3);
        let tree = MerkleTree::build(chain.entries()).unwrap();
        let anchor = WitnessAnchor::from_tree(&tree, "explicit_head");

        assert_eq!(anchor.chain_head, "explicit_head");
        assert_eq!(anchor.merkle_root, tree.root());
    }

    #[test]
    fn anchor_verify_against_valid() {
        let chain = make_chain(5);
        let tree = MerkleTree::build(chain.entries()).unwrap();
        let anchor = WitnessAnchor::new(&tree, &chain).unwrap();

        assert!(anchor.verify_against(&tree, &chain).is_valid());
    }

    #[test]
    fn anchor_verify_root_mismatch() {
        let chain = make_chain(5);
        let tree = MerkleTree::build(chain.entries()).unwrap();
        let anchor = WitnessAnchor::new(&tree, &chain).unwrap();

        // Build a different tree
        let chain2 = make_chain(5); // different UUIDs/timestamps
        let tree2 = MerkleTree::build(chain2.entries()).unwrap();

        let result = anchor.verify_against_tree(&tree2);
        assert!(!result.is_valid());
        assert!(matches!(result, AnchorVerification::RootMismatch { .. }));
    }

    #[test]
    fn anchor_verify_count_mismatch() {
        let chain = make_chain(5);
        let tree = MerkleTree::build(chain.entries()).unwrap();
        let anchor = WitnessAnchor::new(&tree, &chain).unwrap();

        // Build a tree with different count
        let chain2 = make_chain(3);
        let tree2 = MerkleTree::build(chain2.entries()).unwrap();

        let result = anchor.verify_against_tree(&tree2);
        assert!(matches!(result, AnchorVerification::CountMismatch { .. }));
    }

    #[test]
    fn anchor_verify_head_mismatch() {
        let chain1 = make_chain(5);
        let tree = MerkleTree::build(chain1.entries()).unwrap();
        let anchor = WitnessAnchor::new(&tree, &chain1).unwrap();

        // Different chain with different head
        let chain2 = make_chain(5);
        let result = anchor.verify_against(&tree, &chain2);
        assert!(matches!(result, AnchorVerification::HeadMismatch { .. }));
    }

    #[test]
    fn anchor_verify_empty_chain_head_mismatch() {
        let chain = make_chain(3);
        let tree = MerkleTree::build(chain.entries()).unwrap();
        let anchor = WitnessAnchor::new(&tree, &chain).unwrap();

        // Verify against an empty chain — should detect head mismatch
        let empty_chain = AuditChain::new();
        let result = anchor.verify_against(&tree, &empty_chain);
        assert!(matches!(result, AnchorVerification::HeadMismatch { .. }));
    }

    #[test]
    fn anchor_field_boundary_ambiguity() {
        // Anchors with shifted field boundaries should have different hashes
        let chain = make_chain(3);
        let tree = MerkleTree::build(chain.entries()).unwrap();
        let a1 = WitnessAnchor::from_tree(&tree, "ab");
        let a2 = WitnessAnchor::from_tree(&tree, "abc");
        // Same tree, different chain_head → different hashes
        assert_ne!(a1.hash, a2.hash);
    }

    #[test]
    fn anchor_integrity() {
        let chain = make_chain(3);
        let tree = MerkleTree::build(chain.entries()).unwrap();
        let anchor = WitnessAnchor::new(&tree, &chain).unwrap();

        assert!(anchor.verify_integrity());
    }

    #[test]
    fn anchor_integrity_tampered() {
        let chain = make_chain(3);
        let tree = MerkleTree::build(chain.entries()).unwrap();
        let mut anchor = WitnessAnchor::new(&tree, &chain).unwrap();

        anchor.merkle_root = "tampered".to_owned();
        assert!(!anchor.verify_integrity());
    }

    #[test]
    fn anchor_chaining() {
        let chain1 = make_chain(3);
        let tree1 = MerkleTree::build(chain1.entries()).unwrap();
        let anchor1 = WitnessAnchor::new(&tree1, &chain1).unwrap();

        let chain2 = make_chain(5);
        let tree2 = MerkleTree::build(chain2.entries()).unwrap();
        let anchor2 = WitnessAnchor::new(&tree2, &chain2)
            .unwrap()
            .with_prev_anchor(&anchor1);

        assert_eq!(
            anchor2.prev_anchor_hash.as_deref(),
            Some(anchor1.hash.as_str())
        );
        assert!(anchor2.verify_integrity());

        // Anchor 1 and 2 have different hashes
        assert_ne!(anchor1.hash, anchor2.hash);
    }

    #[test]
    fn receipt_creation() {
        let receipt = WitnessReceipt::new(
            Uuid::new_v4(),
            "test-backend",
            serde_json::json!({"tx": "abc123"}),
        );
        assert_eq!(receipt.backend, "test-backend");
        assert!(receipt.receipt_hash.is_none());
    }

    #[test]
    fn receipt_with_hash() {
        let receipt = WitnessReceipt::new(
            Uuid::new_v4(),
            "test-backend",
            serde_json::json!({"tx": "abc123"}),
        )
        .with_hash();
        assert!(receipt.receipt_hash.is_some());
    }

    #[test]
    fn serde_roundtrip_anchor() {
        let chain = make_chain(3);
        let tree = MerkleTree::build(chain.entries()).unwrap();
        let anchor = WitnessAnchor::new(&tree, &chain).unwrap();

        let json = serde_json::to_string(&anchor).unwrap();
        let back: WitnessAnchor = serde_json::from_str(&json).unwrap();
        assert_eq!(back.hash, anchor.hash);
        assert!(back.verify_integrity());
    }

    #[test]
    fn serde_roundtrip_receipt() {
        let receipt = WitnessReceipt::new(Uuid::new_v4(), "test", serde_json::json!({"data": 42}))
            .with_hash();
        let json = serde_json::to_string(&receipt).unwrap();
        let back: WitnessReceipt = serde_json::from_str(&json).unwrap();
        assert_eq!(back.receipt_hash, receipt.receipt_hash);
    }

    #[test]
    fn serde_roundtrip_verification() {
        let v = AnchorVerification::RootMismatch {
            expected: "a".into(),
            actual: "b".into(),
        };
        let json = serde_json::to_string(&v).unwrap();
        let back: AnchorVerification = serde_json::from_str(&json).unwrap();
        assert_eq!(v, back);
    }

    #[test]
    fn verification_display() {
        assert_eq!(AnchorVerification::Valid.to_string(), "valid");
        let m = AnchorVerification::CountMismatch {
            expected: 5,
            actual: 3,
        };
        assert!(m.to_string().contains("5"));
        assert!(m.to_string().contains("3"));
    }
}
