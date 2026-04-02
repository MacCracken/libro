//! Merkle tree for efficient partial verification of audit chains.
//!
//! Given a chain of N entries, build a binary Merkle tree where each leaf
//! is the entry's hash. This enables:
//! - **O(1)** root hash comparison (did anything change?)
//! - **O(log N)** proof generation and verification for a single entry
//! - Verification without access to the full chain
//!
//! The tree uses SHA-256, consistent with the chain's hash algorithm.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::entry::AuditEntry;

/// A Merkle tree built from audit entry hashes.
#[derive(Debug, Clone)]
pub struct MerkleTree {
    /// All nodes, stored level by level bottom-up. Leaves are at the start.
    nodes: Vec<String>,
    /// Number of leaves (entries).
    leaf_count: usize,
}

/// An inclusion proof for a single entry in the Merkle tree.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct MerkleProof {
    /// The entry index this proof is for.
    pub index: usize,
    /// The hash of the entry (leaf).
    pub leaf_hash: String,
    /// Sibling hashes from leaf to root, with position (Left or Right).
    pub path: Vec<ProofNode>,
    /// The expected root hash.
    pub root: String,
}

/// A node in a Merkle proof path.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ProofNode {
    /// The sibling hash.
    pub hash: String,
    /// Whether this sibling is on the left or right.
    pub side: Side,
}

/// Side indicator for proof path nodes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum Side {
    Left,
    Right,
}

impl MerkleTree {
    /// Build a Merkle tree from a slice of audit entries.
    ///
    /// Returns `None` if the entries slice is empty.
    pub fn build(entries: &[AuditEntry]) -> Option<Self> {
        if entries.is_empty() {
            return None;
        }

        let leaves: Vec<String> = entries.iter().map(|e| e.hash().to_owned()).collect();
        let leaf_count = leaves.len();

        // Build tree bottom-up, moving levels into nodes to avoid clones
        let mut current_level = leaves;
        let mut nodes = Vec::new();

        loop {
            if current_level.len() == 1 {
                nodes.extend(current_level);
                break;
            }

            let mut next_level = Vec::with_capacity(current_level.len().div_ceil(2));
            let mut i = 0;
            while i < current_level.len() {
                let left = &current_level[i];
                let right = if i + 1 < current_level.len() {
                    &current_level[i + 1]
                } else {
                    // Odd node: duplicate the last
                    left
                };
                next_level.push(hash_pair(left, right));
                i += 2;
            }
            nodes.extend(current_level);
            current_level = next_level;
        }

        Some(Self { nodes, leaf_count })
    }

    /// The Merkle root hash.
    #[inline]
    #[must_use]
    pub fn root(&self) -> &str {
        // Root is the last node
        self.nodes.last().map(|s| s.as_str()).unwrap_or("")
    }

    /// Number of leaves (entries) in the tree.
    #[inline]
    #[must_use]
    pub fn leaf_count(&self) -> usize {
        self.leaf_count
    }

    /// Generate an inclusion proof for the entry at the given index.
    ///
    /// Returns `None` if the index is out of bounds.
    pub fn proof(&self, index: usize) -> Option<MerkleProof> {
        if index >= self.leaf_count {
            return None;
        }

        let mut path = Vec::new();
        let mut level_start = 0;
        let mut level_size = self.leaf_count;
        let mut idx = index;

        while level_size > 1 {
            let sibling_idx = if idx.is_multiple_of(2) {
                idx + 1
            } else {
                idx - 1
            };

            let sibling_hash = if sibling_idx < level_size {
                self.nodes[level_start + sibling_idx].clone()
            } else {
                // Odd level: sibling is self (duplicated)
                self.nodes[level_start + idx].clone()
            };

            let side = if idx.is_multiple_of(2) {
                Side::Right
            } else {
                Side::Left
            };

            path.push(ProofNode {
                hash: sibling_hash,
                side,
            });

            level_start += level_size;
            level_size = level_size.div_ceil(2);
            idx /= 2;
        }

        Some(MerkleProof {
            index,
            leaf_hash: self.nodes[index].clone(),
            path,
            root: self.root().to_owned(),
        })
    }
}

/// Verify a Merkle inclusion proof.
///
/// Returns `true` if the proof is valid — the leaf hash, combined with
/// the proof path, produces the expected root.
#[must_use]
pub fn verify_proof(proof: &MerkleProof) -> bool {
    let mut current = proof.leaf_hash.clone();

    for node in &proof.path {
        current = match node.side {
            Side::Left => hash_pair(&node.hash, &current),
            Side::Right => hash_pair(&current, &node.hash),
        };
    }

    current == proof.root
}

/// Hash two child nodes to produce a parent node.
#[inline]
fn hash_pair(left: &str, right: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(left.as_bytes());
    hasher.update(right.as_bytes());
    format!("{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entry::EventSeverity;

    fn make_entries(n: usize) -> Vec<AuditEntry> {
        let mut entries = Vec::new();
        let first = AuditEntry::new(EventSeverity::Info, "s", "e0", serde_json::json!({}), "");
        entries.push(first);
        for i in 1..n {
            let prev = entries[i - 1].hash();
            entries.push(AuditEntry::new(
                EventSeverity::Info,
                "s",
                format!("e{i}"),
                serde_json::json!({}),
                prev,
            ));
        }
        entries
    }

    #[test]
    fn build_empty() {
        assert!(MerkleTree::build(&[]).is_none());
    }

    #[test]
    fn build_single_entry() {
        let entries = make_entries(1);
        let tree = MerkleTree::build(&entries).unwrap();
        assert_eq!(tree.leaf_count(), 1);
        // Root is the single leaf hash
        assert_eq!(tree.root(), entries[0].hash());
    }

    #[test]
    fn build_two_entries() {
        let entries = make_entries(2);
        let tree = MerkleTree::build(&entries).unwrap();
        assert_eq!(tree.leaf_count(), 2);
        // Root should be hash of the two entry hashes
        let expected_root = hash_pair(entries[0].hash(), entries[1].hash());
        assert_eq!(tree.root(), expected_root);
    }

    #[test]
    fn build_power_of_two() {
        let entries = make_entries(8);
        let tree = MerkleTree::build(&entries).unwrap();
        assert_eq!(tree.leaf_count(), 8);
        assert!(!tree.root().is_empty());
    }

    #[test]
    fn build_odd_count() {
        let entries = make_entries(5);
        let tree = MerkleTree::build(&entries).unwrap();
        assert_eq!(tree.leaf_count(), 5);
        assert!(!tree.root().is_empty());
    }

    #[test]
    fn proof_and_verify_all_entries() {
        let entries = make_entries(8);
        let tree = MerkleTree::build(&entries).unwrap();

        for (i, entry) in entries.iter().enumerate() {
            let proof = tree.proof(i).unwrap();
            assert_eq!(proof.index, i);
            assert_eq!(proof.leaf_hash, entry.hash());
            assert_eq!(proof.root, tree.root());
            assert!(verify_proof(&proof), "proof failed for index {i}");
        }
    }

    #[test]
    fn proof_and_verify_odd_tree() {
        let entries = make_entries(7);
        let tree = MerkleTree::build(&entries).unwrap();

        for i in 0..entries.len() {
            let proof = tree.proof(i).unwrap();
            assert!(verify_proof(&proof), "proof failed for index {i}");
        }
    }

    #[test]
    fn proof_out_of_bounds() {
        let entries = make_entries(4);
        let tree = MerkleTree::build(&entries).unwrap();
        assert!(tree.proof(4).is_none());
        assert!(tree.proof(100).is_none());
    }

    #[test]
    fn tampered_proof_fails() {
        let entries = make_entries(8);
        let tree = MerkleTree::build(&entries).unwrap();
        let mut proof = tree.proof(3).unwrap();
        proof.leaf_hash = "tampered".to_owned();
        assert!(!verify_proof(&proof));
    }

    #[test]
    fn tampered_path_fails() {
        let entries = make_entries(8);
        let tree = MerkleTree::build(&entries).unwrap();
        let mut proof = tree.proof(3).unwrap();
        if let Some(node) = proof.path.first_mut() {
            node.hash = "tampered".to_owned();
        }
        assert!(!verify_proof(&proof));
    }

    #[test]
    fn different_entries_different_roots() {
        let entries_a = make_entries(4);
        let mut entries_b = make_entries(4);
        // Tamper with one entry in b
        entries_b[2].corrupt_action("different");
        // Recompute chain from scratch won't work since hashes are stale,
        // but the point is the Merkle roots will differ
        let tree_a = MerkleTree::build(&entries_a).unwrap();
        let tree_b = MerkleTree::build(&entries_b).unwrap();
        assert_ne!(tree_a.root(), tree_b.root());
    }

    #[test]
    fn large_tree() {
        let entries = make_entries(100);
        let tree = MerkleTree::build(&entries).unwrap();
        assert_eq!(tree.leaf_count(), 100);

        // Spot-check a few proofs
        for i in [0, 49, 99] {
            let proof = tree.proof(i).unwrap();
            assert!(verify_proof(&proof));
        }
    }

    #[test]
    fn single_entry_proof() {
        let entries = make_entries(1);
        let tree = MerkleTree::build(&entries).unwrap();
        let proof = tree.proof(0).unwrap();
        assert!(proof.path.is_empty()); // No siblings needed
        assert!(verify_proof(&proof));
    }
}
