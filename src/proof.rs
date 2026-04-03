//! Integrity proof export for independent auditor verification.
//!
//! Produces a self-contained [`IntegrityProof`] bundle that an auditor can
//! verify without access to the original chain or storage backend. The bundle
//! includes a signed Merkle tree head, entry data, inclusion proofs, and
//! optional consistency proofs linking to a previous state.
//!
//! # Usage
//!
//! ```rust,ignore
//! use libro::proof::IntegrityProof;
//! use libro::{AuditChain, EventSeverity};
//!
//! let mut chain = AuditChain::new();
//! chain.append(EventSeverity::Info, "src", "act", serde_json::json!({}));
//!
//! // With signing (feature = "signing")
//! let key = libro::signing::SigningKey::generate();
//! let proof = IntegrityProof::builder(&chain, &key)
//!     .unwrap()
//!     .with_all_inclusions()
//!     .build();
//!
//! let result = proof.verify(&key.verifying_key());
//! assert!(result.is_valid());
//! ```

use std::io::Write;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[cfg(feature = "anchoring")]
use crate::anchoring::WitnessAnchor;
use crate::chain::AuditChain;
use crate::entry::AuditEntry;
use crate::hasher::hex_encode_slice;
use crate::merkle::{ConsistencyProof, MerkleProof, MerkleTree, verify_consistency, verify_proof};
#[cfg(feature = "signing")]
use crate::signing::{EntrySigner, EntryVerifier};

/// A signed Merkle tree head — commits to the chain state at a point in time.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct SignedTreeHead {
    /// The Merkle root hash (hex-encoded).
    pub root: String,
    /// Number of entries in the tree.
    pub tree_size: usize,
    /// When the tree head was signed.
    pub timestamp: DateTime<Utc>,
    /// Hex-encoded signature over the root bytes.
    pub signature: String,
    /// Hex-encoded verifying (public) key.
    pub verifying_key: String,
    /// The signature algorithm used (e.g., "ed25519").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub algorithm: Option<String>,
}

/// A self-contained integrity proof bundle for independent verification.
///
/// Contains everything an auditor needs to verify chain integrity without
/// access to the original chain or storage backend.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct IntegrityProof {
    /// The signed Merkle tree head.
    pub tree_head: SignedTreeHead,
    /// RFC 9162 consistency proof from a previous tree size (if any).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub consistency: Option<ConsistencyProof>,
    /// Inclusion proofs for selected entries.
    pub inclusions: Vec<MerkleProof>,
    /// Previous witness anchor (if any).
    #[cfg(feature = "anchoring")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub anchor: Option<WitnessAnchor>,
    /// The audit entries covered by this proof.
    pub entries: Vec<AuditEntry>,
}

/// Detailed verification results for an [`IntegrityProof`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ProofVerification {
    /// Whether the tree head signature is valid.
    pub tree_head_valid: bool,
    /// Whether all entries verify individually and chain correctly.
    pub entries_valid: bool,
    /// Whether the Merkle tree built from entries matches the signed root.
    pub tree_matches: bool,
    /// Whether all inclusion proofs are valid.
    pub inclusions_valid: bool,
    /// Whether the consistency proof is valid (if present).
    pub consistency_valid: Option<bool>,
    /// Whether the anchor integrity check passes (if present).
    #[cfg(feature = "anchoring")]
    pub anchor_valid: Option<bool>,
}

impl ProofVerification {
    /// Whether all checks passed.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.tree_head_valid
            && self.entries_valid
            && self.tree_matches
            && self.inclusions_valid
            && self.consistency_valid.unwrap_or(true)
            && {
                #[cfg(feature = "anchoring")]
                {
                    self.anchor_valid.unwrap_or(true)
                }
                #[cfg(not(feature = "anchoring"))]
                true
            }
    }
}

/// Builder for constructing an [`IntegrityProof`].
pub struct ProofBuilder<'a> {
    chain: &'a AuditChain,
    tree: MerkleTree,
    tree_head: SignedTreeHead,
    consistency: Option<ConsistencyProof>,
    inclusions: Vec<MerkleProof>,
    #[cfg(feature = "anchoring")]
    anchor: Option<WitnessAnchor>,
}

impl IntegrityProof {
    /// Create a proof builder with a signed tree head.
    ///
    /// Returns `None` if the chain is empty.
    #[cfg(feature = "signing")]
    pub fn builder<'a>(
        chain: &'a AuditChain,
        signer: &dyn EntrySigner,
    ) -> Option<ProofBuilder<'a>> {
        let tree = MerkleTree::build(chain.entries())?;
        let root = tree.root().to_owned();
        let sig_bytes = signer.sign_bytes(root.as_bytes());

        let tree_head = SignedTreeHead {
            root,
            tree_size: tree.leaf_count(),
            timestamp: Utc::now(),
            signature: hex_encode_slice(&sig_bytes),
            verifying_key: hex_encode_slice(&signer.verifying_key_bytes()),
            algorithm: Some(signer.algorithm().as_str().to_owned()),
        };

        Some(ProofBuilder {
            chain,
            tree,
            tree_head,
            consistency: None,
            inclusions: Vec::new(),
            #[cfg(feature = "anchoring")]
            anchor: None,
        })
    }

    /// Create a proof builder without signing (unsigned tree head).
    ///
    /// The tree head will have empty signature and verifying key fields.
    /// Useful for testing or when signatures are applied externally.
    ///
    /// Returns `None` if the chain is empty.
    pub fn builder_unsigned(chain: &AuditChain) -> Option<ProofBuilder<'_>> {
        let tree = MerkleTree::build(chain.entries())?;
        let root = tree.root().to_owned();

        let tree_head = SignedTreeHead {
            root,
            tree_size: tree.leaf_count(),
            timestamp: Utc::now(),
            signature: String::new(),
            verifying_key: String::new(),
            algorithm: None,
        };

        Some(ProofBuilder {
            chain,
            tree,
            tree_head,
            consistency: None,
            inclusions: Vec::new(),
            #[cfg(feature = "anchoring")]
            anchor: None,
        })
    }

    /// Verify this proof bundle.
    ///
    /// When `signing` feature is enabled, pass a verifier to check the
    /// tree head signature. Without signing, signature verification is skipped.
    #[cfg(feature = "signing")]
    pub fn verify(&self, verifier: &dyn EntryVerifier) -> ProofVerification {
        self.verify_signed(verifier)
    }

    /// Verify this proof bundle without signature verification.
    pub fn verify_unsigned(&self) -> ProofVerification {
        self.verify_common(true)
    }

    #[cfg(feature = "signing")]
    fn verify_signed(&self, verifier: &dyn EntryVerifier) -> ProofVerification {
        let sig_bytes = crate::hasher::hex_decode(&self.tree_head.signature).unwrap_or_default();
        let tree_head_valid = verifier.verify_bytes(self.tree_head.root.as_bytes(), &sig_bytes);
        self.verify_common(tree_head_valid)
    }

    fn verify_common(&self, tree_head_valid: bool) -> ProofVerification {
        // Entry verification: each entry's hash matches its content + chain links
        let entries_valid = self.verify_entries();

        // Build tree once for reuse across checks
        let rebuilt_tree = MerkleTree::build(&self.entries);

        // Tree matches: compare rebuilt root against signed root
        let tree_matches = rebuilt_tree
            .as_ref()
            .map(|tree| {
                crate::entry::constant_time_eq(tree.root(), &self.tree_head.root)
                    && tree.leaf_count() == self.tree_head.tree_size
            })
            .unwrap_or(self.entries.is_empty() && self.tree_head.tree_size == 0);

        // Inclusion proofs
        let inclusions_valid = self.inclusions.iter().all(|p| {
            verify_proof(p) && crate::entry::constant_time_eq(&p.root, &self.tree_head.root)
        });

        // Consistency proof — compare against canonical root (RFC 9162),
        // which may differ from tree_head.root for non-power-of-2 sizes.
        let consistency_valid = self.consistency.as_ref().map(|c| {
            if !verify_consistency(c) {
                return false;
            }
            rebuilt_tree
                .as_ref()
                .and_then(|tree| tree.canonical_root(tree.leaf_count()))
                .is_some_and(|canonical| crate::entry::constant_time_eq(&c.new_root, &canonical))
        });

        // Anchor integrity
        #[cfg(feature = "anchoring")]
        let anchor_valid = self.anchor.as_ref().map(|a| a.verify_integrity());

        ProofVerification {
            tree_head_valid,
            entries_valid,
            tree_matches,
            inclusions_valid,
            consistency_valid,
            #[cfg(feature = "anchoring")]
            anchor_valid,
        }
    }

    fn verify_entries(&self) -> bool {
        for (i, entry) in self.entries.iter().enumerate() {
            if !entry.verify() {
                return false;
            }
            if i > 0
                && !crate::entry::constant_time_eq(entry.prev_hash(), self.entries[i - 1].hash())
            {
                return false;
            }
        }
        true
    }
}

impl<'a> ProofBuilder<'a> {
    /// Add an RFC 9162 consistency proof from a previous tree size.
    ///
    /// Proves that the first `old_size` entries in the current tree
    /// produce the same root as a tree built from only those entries.
    #[must_use]
    pub fn with_consistency_from(mut self, old_size: usize) -> Self {
        self.consistency = self.tree.consistency_proof(old_size);
        self
    }

    /// Include a witness anchor from a previous state.
    #[cfg(feature = "anchoring")]
    #[must_use]
    pub fn with_anchor(mut self, anchor: &WitnessAnchor) -> Self {
        self.anchor = Some(anchor.clone());
        self
    }

    /// Add an inclusion proof for the entry at the given index.
    #[must_use]
    pub fn with_inclusion(mut self, index: usize) -> Self {
        if let Some(proof) = self.tree.proof(index) {
            self.inclusions.push(proof);
        }
        self
    }

    /// Add inclusion proofs for all entries.
    #[must_use]
    pub fn with_all_inclusions(mut self) -> Self {
        for i in 0..self.tree.leaf_count() {
            if let Some(proof) = self.tree.proof(i) {
                self.inclusions.push(proof);
            }
        }
        self
    }

    /// Build the integrity proof, consuming this builder.
    #[must_use]
    pub fn build(self) -> IntegrityProof {
        IntegrityProof {
            tree_head: self.tree_head,
            consistency: self.consistency,
            inclusions: self.inclusions,
            #[cfg(feature = "anchoring")]
            anchor: self.anchor,
            entries: self.chain.entries().to_vec(),
        }
    }
}

/// Serialize an [`IntegrityProof`] as pretty-printed JSON.
///
/// Follows the same `impl Write` pattern as [`to_jsonl`](crate::to_jsonl)
/// and [`to_csv`](crate::to_csv).
pub fn to_proof_json(proof: &IntegrityProof, mut writer: impl Write) -> crate::Result<()> {
    let json = serde_json::to_string_pretty(proof)?;
    writer.write_all(json.as_bytes())?;
    Ok(())
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
                "audit",
                format!("event.{i}"),
                serde_json::json!({"i": i}),
            );
        }
        chain
    }

    #[test]
    fn unsigned_proof_build_and_verify() {
        let chain = make_chain(5);
        let proof = IntegrityProof::builder_unsigned(&chain)
            .unwrap()
            .with_all_inclusions()
            .build();

        assert_eq!(proof.tree_head.tree_size, 5);
        assert_eq!(proof.entries.len(), 5);
        assert_eq!(proof.inclusions.len(), 5);
        assert!(proof.tree_head.signature.is_empty());

        let result = proof.verify_unsigned();
        assert!(result.entries_valid);
        assert!(result.tree_matches);
        assert!(result.inclusions_valid);
        assert!(result.is_valid());
    }

    #[cfg(feature = "signing")]
    #[test]
    fn signed_proof_build_and_verify() {
        use crate::signing::SigningKey;

        let chain = make_chain(5);
        let key = SigningKey::generate();
        let proof = IntegrityProof::builder(&chain, &key)
            .unwrap()
            .with_all_inclusions()
            .build();

        assert!(!proof.tree_head.signature.is_empty());
        assert_eq!(proof.tree_head.algorithm.as_deref(), Some("ed25519"));

        let result = proof.verify(&key.verifying_key());
        assert!(result.tree_head_valid);
        assert!(result.entries_valid);
        assert!(result.tree_matches);
        assert!(result.inclusions_valid);
        assert!(result.is_valid());
    }

    #[cfg(feature = "signing")]
    #[test]
    fn signed_proof_wrong_key_fails() {
        use crate::signing::SigningKey;

        let chain = make_chain(3);
        let key_a = SigningKey::generate();
        let key_b = SigningKey::generate();
        let proof = IntegrityProof::builder(&chain, &key_a).unwrap().build();

        let result = proof.verify(&key_b.verifying_key());
        assert!(!result.tree_head_valid);
        assert!(!result.is_valid());
    }

    #[test]
    fn proof_with_consistency() {
        let chain = make_chain(10);
        let proof = IntegrityProof::builder_unsigned(&chain)
            .unwrap()
            .with_consistency_from(5)
            .build();

        assert!(proof.consistency.is_some());
        let result = proof.verify_unsigned();
        assert_eq!(result.consistency_valid, Some(true));
        assert!(result.is_valid());
    }

    #[test]
    fn proof_with_selective_inclusions() {
        let chain = make_chain(10);
        let proof = IntegrityProof::builder_unsigned(&chain)
            .unwrap()
            .with_inclusion(0)
            .with_inclusion(5)
            .with_inclusion(9)
            .build();

        assert_eq!(proof.inclusions.len(), 3);
        let result = proof.verify_unsigned();
        assert!(result.inclusions_valid);
        assert!(result.is_valid());
    }

    #[cfg(feature = "anchoring")]
    #[test]
    fn proof_with_anchor() {
        let chain = make_chain(5);
        let tree = MerkleTree::build(chain.entries()).unwrap();
        let anchor = crate::anchoring::WitnessAnchor::new(&tree, &chain).unwrap();

        let proof = IntegrityProof::builder_unsigned(&chain)
            .unwrap()
            .with_anchor(&anchor)
            .build();

        assert!(proof.anchor.is_some());
        let result = proof.verify_unsigned();
        assert_eq!(result.anchor_valid, Some(true));
        assert!(result.is_valid());
    }

    #[test]
    fn tampered_entry_detected() {
        let chain = make_chain(5);
        let mut proof = IntegrityProof::builder_unsigned(&chain).unwrap().build();

        proof.entries[2].corrupt_action("hacked");
        let result = proof.verify_unsigned();
        assert!(!result.entries_valid);
        assert!(!result.is_valid());
    }

    #[test]
    fn tampered_inclusion_detected() {
        let chain = make_chain(5);
        let mut proof = IntegrityProof::builder_unsigned(&chain)
            .unwrap()
            .with_all_inclusions()
            .build();

        proof.inclusions[2].leaf_hash = "tampered".to_owned();
        let result = proof.verify_unsigned();
        assert!(!result.inclusions_valid);
        assert!(!result.is_valid());
    }

    #[test]
    fn tree_mismatch_detected() {
        let chain = make_chain(5);
        let mut proof = IntegrityProof::builder_unsigned(&chain).unwrap().build();

        proof.tree_head.root = "wrong_root".to_owned();
        let result = proof.verify_unsigned();
        assert!(!result.tree_matches);
        assert!(!result.is_valid());
    }

    #[test]
    fn serde_roundtrip() {
        let chain = make_chain(5);
        let proof = IntegrityProof::builder_unsigned(&chain)
            .unwrap()
            .with_consistency_from(3)
            .with_all_inclusions()
            .build();

        let json = serde_json::to_string(&proof).unwrap();
        let back: IntegrityProof = serde_json::from_str(&json).unwrap();
        assert_eq!(proof, back);

        let result = back.verify_unsigned();
        assert!(result.is_valid());
    }

    #[test]
    fn to_proof_json_writes_valid_json() {
        let chain = make_chain(3);
        let proof = IntegrityProof::builder_unsigned(&chain).unwrap().build();

        let mut buf = Vec::new();
        to_proof_json(&proof, &mut buf).unwrap();
        let parsed: serde_json::Value = serde_json::from_slice(&buf).unwrap();
        assert!(parsed.is_object());
        assert!(parsed["tree_head"]["root"].is_string());
        assert!(parsed["entries"].is_array());
    }

    #[test]
    fn empty_chain_returns_none() {
        let chain = AuditChain::new();
        assert!(IntegrityProof::builder_unsigned(&chain).is_none());
    }
}
