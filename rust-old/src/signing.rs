//! Digital signatures for audit entries.
//!
//! Provides per-entry signing and verification with algorithm-agnostic traits.
//! The default implementation uses Ed25519 via `ed25519-dalek`. Consumers can
//! implement the [`EntrySigner`] and [`EntryVerifier`] traits for other algorithms
//! (e.g., ML-DSA for post-quantum security).
//!
//! Requires the `signing` feature flag.
//!
//! # Usage
//!
//! ```rust,ignore
//! use libro::signing::{SigningKey, EntrySignature, EntrySigner};
//! use libro::{AuditEntry, EventSeverity};
//!
//! let key = SigningKey::generate();
//! let entry = AuditEntry::new(EventSeverity::Info, "src", "act", serde_json::json!({}), "");
//! let sig = key.sign_entry(&entry);
//! assert!(sig.verify(&entry, &key.verifying_key()));
//! ```

use ed25519_dalek::{
    Signature, Signer as DalekSigner, SigningKey as DalekSigningKey, Verifier as DalekVerifier,
    VerifyingKey as DalekVerifyingKey,
};
use rand_core::OsRng;
use serde::{Deserialize, Serialize};
use zeroize::Zeroizing;

use crate::entry::AuditEntry;
use crate::hasher::{hex_decode, hex_encode, hex_encode_slice};

// ---------------------------------------------------------------------------
// Signature algorithm identifier
// ---------------------------------------------------------------------------

/// Identifies the cryptographic signature algorithm used.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum SignatureAlgorithm {
    /// Ed25519 (RFC 8032).
    #[serde(rename = "ed25519")]
    Ed25519,
    /// ML-DSA-65 (FIPS 204, formerly CRYSTALS-Dilithium).
    #[serde(rename = "ml-dsa-65")]
    MlDsa65,
    /// Hybrid Ed25519 + ML-DSA-65 (transition period).
    #[serde(rename = "ed25519+ml-dsa-65")]
    Ed25519MlDsa65,
}

impl SignatureAlgorithm {
    /// Stable string representation.
    #[inline]
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ed25519 => "ed25519",
            Self::MlDsa65 => "ml-dsa-65",
            Self::Ed25519MlDsa65 => "ed25519+ml-dsa-65",
        }
    }
}

impl std::fmt::Display for SignatureAlgorithm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for SignatureAlgorithm {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "ed25519" => Ok(Self::Ed25519),
            "ml-dsa-65" => Ok(Self::MlDsa65),
            "ed25519+ml-dsa-65" => Ok(Self::Ed25519MlDsa65),
            other => Err(format!("unknown signature algorithm: {other}")),
        }
    }
}

// ---------------------------------------------------------------------------
// Algorithm-agnostic traits
// ---------------------------------------------------------------------------

/// Algorithm-agnostic signing trait for audit entries.
///
/// Implementors provide the low-level `sign_bytes` operation; the trait
/// provides default implementations for `sign_entry` and `sign_entry_with_key_id`
/// that handle entry hash extraction and `EntrySignature` construction.
pub trait EntrySigner: Send + Sync {
    /// The signature algorithm this signer uses.
    fn algorithm(&self) -> SignatureAlgorithm;

    /// The raw verifying (public) key bytes.
    fn verifying_key_bytes(&self) -> Vec<u8>;

    /// Sign raw message bytes, returning the signature bytes.
    fn sign_bytes(&self, message: &[u8]) -> Vec<u8>;

    /// Sign an audit entry, producing an [`EntrySignature`].
    fn sign_entry(&self, entry: &AuditEntry) -> EntrySignature {
        let hash = entry.hash();
        let sig_bytes = self.sign_bytes(hash.as_bytes());
        EntrySignature {
            entry_hash: hash.to_owned(),
            signature: hex_encode_slice(&sig_bytes),
            verifying_key: hex_encode_slice(&self.verifying_key_bytes()),
            key_id: None,
            algorithm: Some(self.algorithm().as_str().to_owned()),
        }
    }

    /// Sign an audit entry with a key identifier for key rotation workflows.
    fn sign_entry_with_key_id(&self, entry: &AuditEntry, key_id: String) -> EntrySignature {
        let mut sig = self.sign_entry(entry);
        sig.key_id = Some(key_id);
        sig
    }
}

/// Algorithm-agnostic verification trait for audit entry signatures.
///
/// Implementors provide the low-level `verify_bytes` operation; the trait
/// provides a default `verify_entry_signature` that handles entry hash
/// checking and hex decoding.
pub trait EntryVerifier: Send + Sync {
    /// The signature algorithm this verifier supports.
    fn algorithm(&self) -> SignatureAlgorithm;

    /// The raw verifying (public) key bytes.
    fn verifying_key_bytes(&self) -> Vec<u8>;

    /// Verify a signature over raw message bytes.
    fn verify_bytes(&self, message: &[u8], signature: &[u8]) -> bool;

    /// Verify an [`EntrySignature`] against an audit entry.
    fn verify_entry_signature(&self, entry: &AuditEntry, sig: &EntrySignature) -> bool {
        if !crate::entry::constant_time_eq(entry.hash(), &sig.entry_hash) {
            return false;
        }
        let sig_bytes = match hex_decode(&sig.signature) {
            Some(b) => b,
            None => return false,
        };
        self.verify_bytes(sig.entry_hash.as_bytes(), &sig_bytes)
    }
}

/// An Ed25519 signing key for audit entries.
///
/// The key material is automatically zeroized when this value is dropped,
/// preventing sensitive bytes from lingering in memory.
#[derive(Debug)]
pub struct SigningKey {
    inner: DalekSigningKey,
}

impl Drop for SigningKey {
    fn drop(&mut self) {
        // Overwrite the key material with a known-zero key.
        // dalek's SigningKey doesn't expose mutable byte access,
        // so we replace the entire inner value.
        self.inner = DalekSigningKey::from_bytes(&[0u8; 32]);
    }
}

/// An Ed25519 verifying (public) key.
#[derive(Debug, Clone)]
pub struct VerifyingKey {
    inner: DalekVerifyingKey,
}

/// A signature over an audit entry's hash.
///
/// Contains the signature bytes, the entry hash it covers, the verifying key,
/// an optional `key_id` for key rotation, and an optional `algorithm` identifier.
/// All cryptographic material is hex-encoded for safe JSON serialization.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct EntrySignature {
    /// The entry hash that was signed.
    pub entry_hash: String,
    /// The raw signature bytes, hex-encoded.
    pub signature: String,
    /// The verifying key that can validate this signature, hex-encoded.
    pub verifying_key: String,
    /// Optional key identifier for key rotation workflows.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key_id: Option<String>,
    /// The signature algorithm used (e.g., "ed25519", "ml-dsa-65").
    /// `None` for legacy entries (assumed Ed25519).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub algorithm: Option<String>,
}

impl SigningKey {
    /// Generate a new random signing key.
    pub fn generate() -> Self {
        let mut rng = OsRng;
        Self {
            inner: DalekSigningKey::generate(&mut rng),
        }
    }

    /// Reconstruct a signing key from its 32-byte seed.
    pub fn from_bytes(bytes: &[u8; 32]) -> Self {
        Self {
            inner: DalekSigningKey::from_bytes(bytes),
        }
    }

    /// Export the signing key's 32-byte seed.
    ///
    /// The returned value is wrapped in [`Zeroizing`], which clears the
    /// bytes from memory when dropped.
    #[must_use]
    pub fn to_bytes(&self) -> Zeroizing<[u8; 32]> {
        Zeroizing::new(self.inner.to_bytes())
    }

    /// Get the corresponding verifying (public) key.
    #[must_use]
    pub fn verifying_key(&self) -> VerifyingKey {
        VerifyingKey {
            inner: self.inner.verifying_key(),
        }
    }

    /// Sign an audit entry. The signature covers the entry's hash.
    pub fn sign(&self, entry: &AuditEntry) -> EntrySignature {
        let hash = entry.hash();
        let sig: Signature = self.inner.sign(hash.as_bytes());
        EntrySignature {
            entry_hash: hash.to_owned(),
            signature: hex_encode(sig.to_bytes()),
            verifying_key: hex_encode(self.inner.verifying_key().to_bytes()),
            key_id: None,
            algorithm: Some(SignatureAlgorithm::Ed25519.as_str().to_owned()),
        }
    }

    /// Sign an audit entry with a key identifier for key rotation workflows.
    ///
    /// The `key_id` is stored in the signature to help consumers look up
    /// the correct verifying key from a key registry.
    pub fn sign_with_key_id(
        &self,
        entry: &AuditEntry,
        key_id: impl Into<String>,
    ) -> EntrySignature {
        let mut sig = self.sign(entry);
        sig.key_id = Some(key_id.into());
        sig
    }
}

impl VerifyingKey {
    /// Reconstruct a verifying key from its 32-byte representation.
    pub fn from_bytes(bytes: &[u8; 32]) -> Result<Self, crate::LibroError> {
        let key = DalekVerifyingKey::from_bytes(bytes)
            .map_err(|e| crate::LibroError::Store(format!("invalid verifying key: {e}")))?;
        Ok(Self { inner: key })
    }

    /// Export the verifying key's 32 bytes.
    #[must_use]
    pub fn to_bytes(&self) -> [u8; 32] {
        self.inner.to_bytes()
    }

    /// Hex-encoded verifying key for storage/display.
    #[must_use]
    pub fn to_hex(&self) -> String {
        hex_encode(self.inner.to_bytes())
    }
}

impl Serialize for VerifyingKey {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_hex())
    }
}

impl<'de> Deserialize<'de> for VerifyingKey {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let hex = String::deserialize(deserializer)?;
        let bytes = hex_decode(&hex)
            .ok_or_else(|| serde::de::Error::custom("invalid hex for verifying key"))?;
        let array: [u8; 32] = bytes
            .try_into()
            .map_err(|_| serde::de::Error::custom("verifying key must be 32 bytes"))?;
        VerifyingKey::from_bytes(&array).map_err(serde::de::Error::custom)
    }
}

impl EntrySignature {
    /// Verify this signature against an entry and a verifying key.
    ///
    /// Checks that:
    /// 1. The entry's current hash matches the signed hash
    /// 2. The Ed25519 signature is valid for that hash
    #[must_use]
    pub fn verify(&self, entry: &AuditEntry, key: &VerifyingKey) -> bool {
        if !crate::entry::constant_time_eq(entry.hash(), &self.entry_hash) {
            return false;
        }
        let sig_bytes = match hex_decode(&self.signature) {
            Some(b) => b,
            None => return false,
        };
        let sig_array: [u8; 64] = match sig_bytes.try_into() {
            Ok(a) => a,
            Err(_) => return false,
        };
        let sig = Signature::from_bytes(&sig_array);
        key.inner.verify(self.entry_hash.as_bytes(), &sig).is_ok()
    }

    /// Verify this signature using any algorithm via the [`EntryVerifier`] trait.
    ///
    /// This enables algorithm-agnostic verification — the verifier determines
    /// the cryptographic algorithm at runtime.
    #[must_use]
    pub fn verify_with(&self, entry: &AuditEntry, verifier: &dyn EntryVerifier) -> bool {
        verifier.verify_entry_signature(entry, self)
    }

    /// Parse the `algorithm` field into a [`SignatureAlgorithm`].
    ///
    /// Returns `None` if the field is absent (legacy entry) or contains
    /// an unrecognized algorithm string.
    #[must_use]
    pub fn algorithm_parsed(&self) -> Option<SignatureAlgorithm> {
        self.algorithm.as_deref()?.parse().ok()
    }
}

// ---------------------------------------------------------------------------
// Trait implementations for Ed25519
// ---------------------------------------------------------------------------

impl EntrySigner for SigningKey {
    fn algorithm(&self) -> SignatureAlgorithm {
        SignatureAlgorithm::Ed25519
    }

    fn verifying_key_bytes(&self) -> Vec<u8> {
        self.inner.verifying_key().to_bytes().to_vec()
    }

    fn sign_bytes(&self, message: &[u8]) -> Vec<u8> {
        let sig: Signature = self.inner.sign(message);
        sig.to_bytes().to_vec()
    }
}

impl EntryVerifier for VerifyingKey {
    fn algorithm(&self) -> SignatureAlgorithm {
        SignatureAlgorithm::Ed25519
    }

    fn verifying_key_bytes(&self) -> Vec<u8> {
        self.inner.to_bytes().to_vec()
    }

    fn verify_bytes(&self, message: &[u8], signature: &[u8]) -> bool {
        let sig_array: [u8; 64] = match signature.try_into() {
            Ok(a) => a,
            Err(_) => return false,
        };
        let sig = Signature::from_bytes(&sig_array);
        self.inner.verify(message, &sig).is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entry::EventSeverity;

    #[test]
    fn sign_and_verify() {
        let key = SigningKey::generate();
        let entry = AuditEntry::new(
            EventSeverity::Info,
            "daimon",
            "start",
            serde_json::json!({}),
            "",
        );
        let sig = key.sign(&entry);

        assert!(sig.verify(&entry, &key.verifying_key()));
    }

    #[test]
    fn verify_fails_for_wrong_key() {
        let key_a = SigningKey::generate();
        let key_b = SigningKey::generate();
        let entry = AuditEntry::new(EventSeverity::Info, "s", "a", serde_json::json!({}), "");
        let sig = key_a.sign(&entry);

        assert!(!sig.verify(&entry, &key_b.verifying_key()));
    }

    #[test]
    fn verify_fails_for_tampered_entry() {
        let key = SigningKey::generate();
        let entry = AuditEntry::new(EventSeverity::Info, "s", "a", serde_json::json!({}), "");
        let sig = key.sign(&entry);

        // Different entry with different hash
        let other = AuditEntry::new(EventSeverity::Info, "s", "b", serde_json::json!({}), "");
        assert!(!sig.verify(&other, &key.verifying_key()));
    }

    #[test]
    fn verify_fails_for_tampered_signature() {
        let key = SigningKey::generate();
        let entry = AuditEntry::new(EventSeverity::Info, "s", "a", serde_json::json!({}), "");
        let mut sig = key.sign(&entry);
        sig.signature = "00".repeat(64);
        assert!(!sig.verify(&entry, &key.verifying_key()));
    }

    #[test]
    fn key_roundtrip() {
        let key = SigningKey::generate();
        let bytes = key.to_bytes();
        let restored = SigningKey::from_bytes(&bytes);
        assert_eq!(
            key.verifying_key().to_hex(),
            restored.verifying_key().to_hex()
        );
    }

    #[test]
    fn verifying_key_roundtrip() {
        let key = SigningKey::generate();
        let vk = key.verifying_key();
        let bytes = vk.to_bytes();
        let restored = VerifyingKey::from_bytes(&bytes).unwrap();
        assert_eq!(vk.to_hex(), restored.to_hex());
    }

    #[test]
    fn sign_chain_entries() {
        let key = SigningKey::generate();
        let vk = key.verifying_key();

        let e1 = AuditEntry::new(EventSeverity::Info, "s", "a", serde_json::json!({}), "");
        let e2 = AuditEntry::new(
            EventSeverity::Info,
            "s",
            "b",
            serde_json::json!({}),
            e1.hash(),
        );

        let sig1 = key.sign(&e1);
        let sig2 = key.sign(&e2);

        assert!(sig1.verify(&e1, &vk));
        assert!(sig2.verify(&e2, &vk));

        // Cross-verify should fail
        assert!(!sig1.verify(&e2, &vk));
        assert!(!sig2.verify(&e1, &vk));
    }

    #[test]
    fn signature_contains_entry_hash() {
        let key = SigningKey::generate();
        let entry = AuditEntry::new(EventSeverity::Info, "s", "a", serde_json::json!({}), "");
        let sig = key.sign(&entry);
        assert_eq!(sig.entry_hash, entry.hash());
    }

    #[test]
    fn invalid_signature_hex() {
        let key = SigningKey::generate();
        let entry = AuditEntry::new(EventSeverity::Info, "s", "a", serde_json::json!({}), "");
        let mut sig = key.sign(&entry);

        // Bad hex
        sig.signature = "not-hex".to_owned();
        assert!(!sig.verify(&entry, &key.verifying_key()));

        // Wrong length
        sig.signature = "ab".to_owned();
        assert!(!sig.verify(&entry, &key.verifying_key()));
    }

    #[test]
    fn invalid_verifying_key_bytes() {
        // All zeros is not a valid ed25519 point
        let result = VerifyingKey::from_bytes(&[0u8; 32]);
        // May or may not error depending on the curve point — just check it doesn't panic
        let _ = result;
    }

    #[test]
    fn sign_with_key_id() {
        let key = SigningKey::generate();
        let entry = AuditEntry::new(EventSeverity::Info, "s", "a", serde_json::json!({}), "");

        // Without key_id
        let sig = key.sign(&entry);
        assert!(sig.key_id.is_none());

        // With key_id
        let sig = key.sign_with_key_id(&entry, "key-v2");
        assert_eq!(sig.key_id.as_deref(), Some("key-v2"));
        assert!(sig.verify(&entry, &key.verifying_key()));

        // Serde roundtrip preserves key_id
        let json = serde_json::to_string(&sig).unwrap();
        assert!(json.contains("key_id"));
        let back: EntrySignature = serde_json::from_str(&json).unwrap();
        assert_eq!(back.key_id.as_deref(), Some("key-v2"));
    }

    #[test]
    fn key_id_skipped_when_none() {
        let key = SigningKey::generate();
        let entry = AuditEntry::new(EventSeverity::Info, "s", "a", serde_json::json!({}), "");
        let sig = key.sign(&entry);
        let json = serde_json::to_string(&sig).unwrap();
        assert!(!json.contains("key_id")); // skipped when None
    }

    #[test]
    fn to_bytes_returns_zeroizing() {
        let key = SigningKey::generate();
        let bytes = key.to_bytes();
        let _: &[u8; 32] = &bytes;
        let restored = SigningKey::from_bytes(&bytes);
        assert_eq!(
            key.verifying_key().to_hex(),
            restored.verifying_key().to_hex()
        );
    }

    // --- Trait-based signing tests ---

    #[test]
    fn trait_sign_and_verify() {
        let key = SigningKey::generate();
        let entry = AuditEntry::new(EventSeverity::Info, "s", "a", serde_json::json!({}), "");

        let sig = EntrySigner::sign_entry(&key, &entry);
        let vk = key.verifying_key();
        assert!(EntryVerifier::verify_entry_signature(&vk, &entry, &sig));
    }

    #[test]
    fn trait_wrong_key_fails() {
        let key_a = SigningKey::generate();
        let key_b = SigningKey::generate();
        let entry = AuditEntry::new(EventSeverity::Info, "s", "a", serde_json::json!({}), "");

        let sig = EntrySigner::sign_entry(&key_a, &entry);
        assert!(!EntryVerifier::verify_entry_signature(
            &key_b.verifying_key(),
            &entry,
            &sig
        ));
    }

    #[test]
    fn algorithm_field_present() {
        let key = SigningKey::generate();
        let entry = AuditEntry::new(EventSeverity::Info, "s", "a", serde_json::json!({}), "");
        let sig = key.sign(&entry);
        assert_eq!(sig.algorithm.as_deref(), Some("ed25519"));
    }

    #[test]
    fn algorithm_field_serde_roundtrip() {
        let key = SigningKey::generate();
        let entry = AuditEntry::new(EventSeverity::Info, "s", "a", serde_json::json!({}), "");
        let sig = key.sign(&entry);

        let json = serde_json::to_string(&sig).unwrap();
        assert!(json.contains("\"algorithm\":\"ed25519\""));
        let back: EntrySignature = serde_json::from_str(&json).unwrap();
        assert_eq!(back.algorithm, sig.algorithm);
    }

    #[test]
    fn algorithm_field_missing_backward_compat() {
        // Simulate a legacy JSON without the algorithm field
        let json = r#"{"entry_hash":"abc","signature":"def","verifying_key":"012"}"#;
        let sig: EntrySignature = serde_json::from_str(json).unwrap();
        assert!(sig.algorithm.is_none());
        assert!(sig.key_id.is_none());
    }

    #[test]
    fn algorithm_parsed() {
        let key = SigningKey::generate();
        let entry = AuditEntry::new(EventSeverity::Info, "s", "a", serde_json::json!({}), "");
        let sig = key.sign(&entry);
        assert_eq!(sig.algorithm_parsed(), Some(SignatureAlgorithm::Ed25519));

        // Missing algorithm
        let mut sig2 = sig.clone();
        sig2.algorithm = None;
        assert_eq!(sig2.algorithm_parsed(), None);

        // Unknown algorithm
        sig2.algorithm = Some("unknown-algo".into());
        assert_eq!(sig2.algorithm_parsed(), None);
    }

    #[test]
    fn signature_algorithm_display_roundtrip() {
        for alg in [
            SignatureAlgorithm::Ed25519,
            SignatureAlgorithm::MlDsa65,
            SignatureAlgorithm::Ed25519MlDsa65,
        ] {
            let s = alg.to_string();
            let parsed: SignatureAlgorithm = s.parse().unwrap();
            assert_eq!(alg, parsed);
        }
    }

    #[test]
    fn verify_with_matches_verify() {
        let key = SigningKey::generate();
        let vk = key.verifying_key();
        let entry = AuditEntry::new(EventSeverity::Info, "s", "a", serde_json::json!({}), "");
        let sig = key.sign(&entry);

        let concrete_result = sig.verify(&entry, &vk);
        let trait_result = sig.verify_with(&entry, &vk);
        assert_eq!(concrete_result, trait_result);
        assert!(concrete_result);
    }

    #[test]
    fn dyn_verifier_works() {
        let key = SigningKey::generate();
        let vk = key.verifying_key();
        let entry = AuditEntry::new(EventSeverity::Info, "s", "a", serde_json::json!({}), "");
        let sig = key.sign(&entry);

        // Use as dyn trait object
        let boxed: Box<dyn EntryVerifier> = Box::new(vk);
        assert!(sig.verify_with(&entry, boxed.as_ref()));
    }

    #[test]
    fn trait_sign_entry_with_key_id() {
        let key = SigningKey::generate();
        let entry = AuditEntry::new(EventSeverity::Info, "s", "a", serde_json::json!({}), "");

        let sig = EntrySigner::sign_entry_with_key_id(&key, &entry, "key-v2".to_owned());
        assert_eq!(sig.key_id.as_deref(), Some("key-v2"));
        assert_eq!(sig.algorithm.as_deref(), Some("ed25519"));
        assert!(sig.verify_with(&entry, &key.verifying_key()));
    }
}
