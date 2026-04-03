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

use crate::entry::AuditEntry;
use crate::hasher::ChainHasher;

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

    /// Generate an RFC 9162 consistency proof from an older tree size.
    ///
    /// Proves that the first `old_size` leaves of this tree produce the same
    /// root as a tree built from only those leaves. This demonstrates the
    /// append-only property.
    ///
    /// Returns `None` if `old_size` is 0 or greater than `leaf_count`.
    pub fn consistency_proof(&self, old_size: usize) -> Option<ConsistencyProof> {
        if old_size == 0 || old_size > self.leaf_count {
            return None;
        }

        let old_root = self.canonical_root(old_size)?;
        let new_root = self.canonical_root(self.leaf_count)?;

        if old_size == self.leaf_count {
            return Some(ConsistencyProof {
                old_size,
                new_size: self.leaf_count,
                old_root,
                new_root,
                path: Vec::new(),
            });
        }

        let mut path = Vec::new();
        subproof(old_size, 0, self.leaf_count, true, &self.nodes, &mut path);

        Some(ConsistencyProof {
            old_size,
            new_size: self.leaf_count,
            old_root,
            new_root,
            path,
        })
    }

    /// Compute the canonical RFC 9162 Merkle root for the first `size` leaves.
    ///
    /// This uses the no-duplication algorithm from RFC 9162: when a level has
    /// an odd number of nodes, the last node is promoted directly (not duplicated).
    /// For power-of-2 sizes, this matches [`root()`]. For others, it may differ.
    ///
    /// Returns `None` if `size` is 0 or greater than `leaf_count`.
    #[must_use]
    pub fn canonical_root(&self, size: usize) -> Option<String> {
        if size == 0 || size > self.leaf_count {
            return None;
        }
        Some(canonical_subtree_hash(&self.nodes, 0, size))
    }
}

/// An RFC 9162 consistency proof demonstrating that a smaller tree is a
/// prefix of a larger tree (append-only property).
///
/// Given tree sizes `old_size` < `new_size`, the proof contains O(log n) hashes
/// that allow reconstructing both the old and new roots. This proves the log
/// has not been tampered with retroactively.
///
/// Uses the canonical (no-duplication) Merkle root computation per RFC 9162.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ConsistencyProof {
    /// The size of the older (smaller) tree.
    pub old_size: usize,
    /// The size of the newer (larger) tree.
    pub new_size: usize,
    /// The canonical RFC 9162 root of the old tree.
    pub old_root: String,
    /// The canonical RFC 9162 root of the new tree.
    pub new_root: String,
    /// Subtree root hashes forming the proof path.
    pub path: Vec<String>,
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

    crate::entry::constant_time_eq(&current, &proof.root)
}

/// Verify an RFC 9162 consistency proof.
///
/// Returns `true` if the proof is valid — the path hashes, combined with
/// the tree sizes, produce both the expected old and new roots.
///
/// Implements RFC 9162 Section 2.1.4.2 verification algorithm.
#[must_use]
pub fn verify_consistency(proof: &ConsistencyProof) -> bool {
    if proof.old_size == 0 || proof.old_size > proof.new_size {
        return false;
    }
    if proof.old_size == proof.new_size {
        return proof.path.is_empty()
            && crate::entry::constant_time_eq(&proof.old_root, &proof.new_root);
    }

    // Step 1: If old_size is a power of 2, prepend old_root to the proof.
    let mut path: Vec<&str> = proof.path.iter().map(|s| s.as_str()).collect();
    if proof.old_size.is_power_of_two() {
        path.insert(0, &proof.old_root);
    }

    if path.is_empty() {
        return false;
    }

    // Step 2: Set fn and sn to tree indices.
    let mut fn_idx = proof.old_size - 1;
    let mut sn_idx = proof.new_size - 1;

    // Step 3: Right-shift both while LSB(fn) is set.
    while fn_idx & 1 == 1 {
        fn_idx >>= 1;
        sn_idx >>= 1;
    }

    // Step 4: Set both fr and sr to the first proof element.
    let mut fr = path[0].to_owned();
    let mut sr = path[0].to_owned();

    // Step 5: For each subsequent value c in the proof.
    for c in &path[1..] {
        // Step 5a: If sn is 0, fail.
        if sn_idx == 0 {
            return false;
        }

        // Step 5b: If LSB(fn) is set, or fn == sn.
        if fn_idx & 1 == 1 || fn_idx == sn_idx {
            // 5b.i-ii: hash(c, fr) and hash(c, sr) — left sibling
            fr = hash_pair(c, &fr);
            sr = hash_pair(c, &sr);

            // 5b.iii: While LSB(fn) is NOT set, shift both.
            while fn_idx != 0 && fn_idx & 1 == 0 {
                fn_idx >>= 1;
                sn_idx >>= 1;
            }
        } else {
            // Step 5c: hash(sr, c) — right sibling (only affects sr)
            sr = hash_pair(&sr, c);
        }

        // Step 5d: Shift both.
        fn_idx >>= 1;
        sn_idx >>= 1;
    }

    // Step 6: Verify sn is 0, fr matches old root, sr matches new root.
    sn_idx == 0
        && crate::entry::constant_time_eq(&fr, &proof.old_root)
        && crate::entry::constant_time_eq(&sr, &proof.new_root)
}

/// RFC 9162 SUBPROOF: collect subtree root hashes for a consistency proof.
///
/// `m`: old tree size within this subtree
/// `start`: starting leaf index in the full tree
/// `n`: subtree size (number of leaves)
/// `is_complete`: whether this subtree is part of the old tree's complete prefix
/// `nodes`: the full tree's node storage
/// `path`: output — proof hashes are appended here
fn subproof(
    m: usize,
    start: usize,
    n: usize,
    is_complete: bool,
    nodes: &[String],
    path: &mut Vec<String>,
) {
    if m == n {
        if !is_complete {
            // Need this subtree's root in the proof
            path.push(canonical_subtree_hash(nodes, start, n));
        }
        return;
    }
    if n == 1 {
        // Single leaf
        if !is_complete {
            path.push(nodes[start].clone());
        }
        return;
    }

    // k = largest power of 2 less than n
    let k = largest_power_of_2_less_than(n);

    if m <= k {
        // Old tree fits entirely in the left subtree
        subproof(m, start, k, is_complete, nodes, path);
        // Right subtree root is part of the proof
        path.push(canonical_subtree_hash(nodes, start + k, n - k));
    } else {
        // Old tree spans into the right subtree
        subproof(m - k, start + k, n - k, false, nodes, path);
        // Left subtree root is part of the proof
        path.push(canonical_subtree_hash(nodes, start, k));
    }
}

/// Compute the canonical RFC 9162 Merkle root for a contiguous range of leaves.
///
/// Uses the no-duplication algorithm: when a level has an odd node count,
/// the last node is promoted directly rather than duplicated.
fn canonical_subtree_hash(nodes: &[String], start: usize, count: usize) -> String {
    if count == 0 {
        return String::new();
    }
    if count == 1 {
        return nodes[start].clone();
    }

    let k = largest_power_of_2_less_than(count);
    let left = canonical_subtree_hash(nodes, start, k);
    let right = canonical_subtree_hash(nodes, start + k, count - k);
    hash_pair(&left, &right)
}

/// Largest power of 2 strictly less than n.
#[inline]
fn largest_power_of_2_less_than(n: usize) -> usize {
    debug_assert!(n > 1);
    1 << (usize::BITS - 1 - (n - 1).leading_zeros())
}

/// Hash two child nodes to produce a parent node.
#[inline]
fn hash_pair(left: &str, right: &str) -> String {
    let mut hasher = ChainHasher::new();
    hasher.update(left.as_bytes());
    hasher.update(right.as_bytes());
    hasher.finalize_hex()
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

    // --- Consistency proof tests ---

    #[test]
    fn consistency_same_size() {
        let entries = make_entries(8);
        let tree = MerkleTree::build(&entries).unwrap();
        let proof = tree.consistency_proof(8).unwrap();
        assert!(proof.path.is_empty());
        assert_eq!(proof.old_root, proof.new_root);
        assert!(verify_consistency(&proof));
    }

    #[test]
    fn consistency_power_of_two() {
        let entries = make_entries(8);
        let tree = MerkleTree::build(&entries).unwrap();

        for old_size in 1..=8 {
            let proof = tree.consistency_proof(old_size).unwrap();
            assert_eq!(proof.old_size, old_size);
            assert_eq!(proof.new_size, 8);
            assert!(
                verify_consistency(&proof),
                "consistency proof failed for old_size={old_size}"
            );
        }
    }

    #[test]
    fn consistency_odd_sizes() {
        for n in [3, 5, 7, 9, 11, 13, 15] {
            let entries = make_entries(n);
            let tree = MerkleTree::build(&entries).unwrap();

            for m in 1..=n {
                let proof = tree.consistency_proof(m).unwrap();
                assert!(
                    verify_consistency(&proof),
                    "consistency proof failed for m={m}, n={n}"
                );
            }
        }
    }

    #[test]
    fn consistency_one_to_many() {
        let entries = make_entries(16);
        let tree = MerkleTree::build(&entries).unwrap();
        let proof = tree.consistency_proof(1).unwrap();
        assert!(verify_consistency(&proof));
        // Single leaf canonical root is the leaf hash itself
        assert_eq!(proof.old_root, entries[0].hash());
    }

    #[test]
    fn consistency_invalid_old_size() {
        let entries = make_entries(5);
        let tree = MerkleTree::build(&entries).unwrap();
        assert!(tree.consistency_proof(0).is_none());
        assert!(tree.consistency_proof(6).is_none());
    }

    #[test]
    fn consistency_tampered_path_fails() {
        let entries = make_entries(8);
        let tree = MerkleTree::build(&entries).unwrap();
        let mut proof = tree.consistency_proof(3).unwrap();
        if let Some(h) = proof.path.first_mut() {
            *h = "tampered".to_owned();
        }
        assert!(!verify_consistency(&proof));
    }

    #[test]
    fn consistency_wrong_old_size_fails() {
        let entries = make_entries(8);
        let tree = MerkleTree::build(&entries).unwrap();
        let mut proof = tree.consistency_proof(4).unwrap();
        proof.old_size = 3; // lie about the old size
        assert!(!verify_consistency(&proof));
    }

    #[test]
    fn canonical_root_power_of_two_matches_tree_root() {
        for n in [1, 2, 4, 8, 16, 32] {
            let entries = make_entries(n);
            let tree = MerkleTree::build(&entries).unwrap();
            let canonical = tree.canonical_root(n).unwrap();
            assert_eq!(
                canonical,
                tree.root(),
                "canonical root should match tree root for power-of-2 size {n}"
            );
        }
    }

    #[test]
    fn canonical_root_bounds() {
        let entries = make_entries(5);
        let tree = MerkleTree::build(&entries).unwrap();
        assert!(tree.canonical_root(0).is_none());
        assert!(tree.canonical_root(6).is_none());
        assert!(tree.canonical_root(5).is_some());
    }

    #[test]
    fn canonical_root_prefix_stable() {
        // The canonical root of the first m leaves should be the same
        // regardless of what comes after
        let entries_5 = make_entries(5);
        let entries_8 = {
            let mut v = entries_5.clone();
            let prev = v.last().unwrap().hash().to_owned();
            for i in 5..8 {
                v.push(AuditEntry::new(
                    EventSeverity::Info,
                    "s",
                    format!("e{i}"),
                    serde_json::json!({}),
                    &prev,
                ));
            }
            v
        };

        let tree_5 = MerkleTree::build(&entries_5).unwrap();
        let tree_8 = MerkleTree::build(&entries_8).unwrap();

        // canonical_root(5) on tree_8 should equal canonical_root(5) on tree_5
        assert_eq!(tree_5.canonical_root(5), tree_8.canonical_root(5));
    }

    #[test]
    fn consistency_large_tree() {
        let entries = make_entries(100);
        let tree = MerkleTree::build(&entries).unwrap();

        // Spot-check several old sizes
        for m in [1, 10, 33, 50, 64, 99, 100] {
            let proof = tree.consistency_proof(m).unwrap();
            assert!(
                verify_consistency(&proof),
                "consistency proof failed for m={m}, n=100"
            );
        }
    }

    #[test]
    fn consistency_serde_roundtrip() {
        let entries = make_entries(8);
        let tree = MerkleTree::build(&entries).unwrap();
        let proof = tree.consistency_proof(3).unwrap();

        let json = serde_json::to_string(&proof).unwrap();
        let back: ConsistencyProof = serde_json::from_str(&json).unwrap();
        assert_eq!(proof, back);
        assert!(verify_consistency(&back));
    }
}
