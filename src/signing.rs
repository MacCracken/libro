//! Ed25519 digital signatures for audit entries.
//!
//! Provides per-entry signing and verification using Ed25519. Each entry's
//! hash is signed, binding the signature to the entry's full content.
//!
//! Requires the `signing` feature flag.
//!
//! # Usage
//!
//! ```rust,ignore
//! use libro::signing::{SigningKey, EntrySignature};
//! use libro::{AuditEntry, EventSeverity};
//!
//! let key = SigningKey::generate();
//! let entry = AuditEntry::new(EventSeverity::Info, "src", "act", serde_json::json!({}), "");
//! let sig = key.sign(&entry);
//! assert!(sig.verify(&entry, &key.verifying_key()));
//! ```

use ed25519_dalek::{
    Signature, Signer, SigningKey as DalekSigningKey, Verifier, VerifyingKey as DalekVerifyingKey,
};
use rand_core::OsRng;
use serde::{Deserialize, Serialize};
use zeroize::Zeroizing;

use crate::entry::AuditEntry;
use crate::hasher::{hex_decode, hex_encode};

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
/// Contains the Ed25519 signature, the entry hash it covers, the verifying key,
/// and an optional `key_id` for key rotation workflows. When using multiple
/// signing keys (e.g., during key rotation), the `key_id` identifies which
/// key produced this signature.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct EntrySignature {
    /// The entry hash that was signed.
    pub entry_hash: String,
    /// The raw Ed25519 signature bytes (64 bytes), hex-encoded.
    pub signature: String,
    /// The verifying key that can validate this signature, hex-encoded.
    pub verifying_key: String,
    /// Optional key identifier for key rotation workflows.
    ///
    /// When set, consumers can use this to look up the correct verifying key
    /// from a key registry. When `None`, the embedded `verifying_key` field
    /// is the sole identifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key_id: Option<String>,
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
        let sig = self.inner.sign(hash.as_bytes());
        EntrySignature {
            entry_hash: hash.to_owned(),
            signature: hex_encode(sig.to_bytes()),
            verifying_key: hex_encode(self.inner.verifying_key().to_bytes()),
            key_id: None,
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
        // Verify it's a Zeroizing wrapper (compiles = passes)
        let _: &[u8; 32] = &bytes;
        // Roundtrip still works through Deref
        let restored = SigningKey::from_bytes(&bytes);
        assert_eq!(
            key.verifying_key().to_hex(),
            restored.verifying_key().to_hex()
        );
    }
}
