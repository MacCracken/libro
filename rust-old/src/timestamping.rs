//! RFC 3161 trusted timestamping for audit chains.
//!
//! Provides data types and construction helpers for RFC 3161 timestamp
//! requests and responses. This module does **not** include an HTTP client —
//! consumers handle transport to the TSA themselves.
//!
//! Requires the `timestamping` feature flag.
//!
//! # Workflow
//!
//! 1. Build a [`TimestampRequest`] from a Merkle root, entry hash, or chain head
//! 2. Serialize to DER via [`to_der()`](TimestampRequest::to_der)
//! 3. POST the DER bytes to a TSA endpoint (`Content-Type: application/timestamp-query`)
//! 4. Parse the response with [`TimestampResponse::from_der()`]
//! 5. Store as a [`TimestampAttestation`] for later verification
//!
//! # Usage
//!
//! ```rust,ignore
//! use libro::timestamping::{TimestampRequest, TimestampResponse, TimestampAttestation};
//! use libro::MerkleTree;
//!
//! let tree = MerkleTree::build(chain.entries()).unwrap();
//! let req = TimestampRequest::from_merkle_root(&tree).with_nonce();
//! let der_bytes = req.to_der();
//!
//! // Consumer: POST der_bytes to TSA, receive response_bytes
//! // let resp = TimestampResponse::from_der(&response_bytes)?;
//! // let attestation = TimestampAttestation::for_merkle_root(&tree, &resp)?;
//! ```

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::LibroError;
use crate::chain::AuditChain;
use crate::entry::{AuditEntry, constant_time_eq};
use crate::hasher::hex_encode_slice;
use crate::merkle::MerkleTree;

// --- DER tag constants ---
const DER_SEQUENCE: u8 = 0x30;
const DER_INTEGER: u8 = 0x02;
const DER_OCTET_STRING: u8 = 0x04;
const DER_BOOLEAN: u8 = 0x01;
const DER_OID: u8 = 0x06;

// OID bytes (pre-encoded, without tag/length)
/// SHA-256: 2.16.840.1.101.3.4.2.1
const OID_SHA256: &[u8] = &[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x02, 0x01];
/// SHA-384: 2.16.840.1.101.3.4.2.2
const OID_SHA384: &[u8] = &[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x02, 0x02];
/// SHA-512: 2.16.840.1.101.3.4.2.3
const OID_SHA512: &[u8] = &[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x02, 0x03];

/// Hash algorithm identifier for timestamp requests.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum TimestampHashAlgorithm {
    /// SHA-256 (OID: 2.16.840.1.101.3.4.2.1)
    Sha256,
    /// SHA-384 (OID: 2.16.840.1.101.3.4.2.2)
    Sha384,
    /// SHA-512 (OID: 2.16.840.1.101.3.4.2.3)
    Sha512,
}

impl TimestampHashAlgorithm {
    fn oid_bytes(self) -> &'static [u8] {
        match self {
            Self::Sha256 => OID_SHA256,
            Self::Sha384 => OID_SHA384,
            Self::Sha512 => OID_SHA512,
        }
    }
}

/// Identifies what kind of data a timestamp covers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum TimestampSubject {
    /// A single audit entry hash.
    EntryHash,
    /// A Merkle root covering a range of entries.
    MerkleRoot,
    /// A chain head hash (snapshot of chain state).
    ChainHead,
}

/// PKI status codes from RFC 3161 Section 2.4.2.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum TimestampStatus {
    /// The request was granted (status 0).
    Granted,
    /// The request was granted with modifications (status 1).
    GrantedWithMods,
    /// The request was rejected (status 2).
    Rejection,
    /// Waiting for approval (status 3).
    Waiting,
    /// Revocation warning (status 4).
    RevocationWarning,
    /// Revocation notification (status 5).
    RevocationNotification,
}

impl TimestampStatus {
    fn from_value(v: u64) -> Option<Self> {
        match v {
            0 => Some(Self::Granted),
            1 => Some(Self::GrantedWithMods),
            2 => Some(Self::Rejection),
            3 => Some(Self::Waiting),
            4 => Some(Self::RevocationWarning),
            5 => Some(Self::RevocationNotification),
            _ => None,
        }
    }
}

/// An RFC 3161 `TimeStampReq`, ready to send to a TSA.
///
/// Serialize to DER via [`to_der()`](TimestampRequest::to_der) and POST
/// to a TSA endpoint with `Content-Type: application/timestamp-query`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct TimestampRequest {
    /// The hash algorithm used to produce `message_imprint_hash`.
    pub hash_algorithm: TimestampHashAlgorithm,
    /// The hex-encoded hash of the data being timestamped.
    pub message_imprint_hash: String,
    /// Optional nonce for replay protection (hex-encoded).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nonce: Option<String>,
    /// Whether to request the TSA's certificate in the response.
    pub cert_req: bool,
}

/// A parsed RFC 3161 `TimeStampResp` from a TSA.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct TimestampResponse {
    /// The PKI status from the TSA.
    pub status: TimestampStatus,
    /// The timestamp token (DER-encoded CMS `ContentInfo`), hex-encoded.
    /// Present only when `status` is `Granted` or `GrantedWithMods`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token: Option<String>,
    /// Optional status string from the TSA.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status_string: Option<String>,
}

/// A stored timestamp attestation binding a hash to a point in time.
///
/// Pairs a libro hash (entry or Merkle root) with the RFC 3161 token
/// and metadata about what was timestamped.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct TimestampAttestation {
    /// The hex-encoded hash that was timestamped.
    pub hash: String,
    /// What kind of hash this is.
    pub hash_subject: TimestampSubject,
    /// The hash algorithm used in the request.
    pub hash_algorithm: TimestampHashAlgorithm,
    /// The raw DER-encoded timestamp token from the TSA, hex-encoded.
    pub token_der: String,
    /// The TSA URL used (informational, for re-verification).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tsa_url: Option<String>,
    /// When the attestation was created (local time, not TSA time).
    pub created_at: DateTime<Utc>,
}

// --- TimestampRequest ---

impl TimestampRequest {
    /// Create a timestamp request for a hex-encoded hash.
    ///
    /// Uses SHA-256 by default and requests the TSA certificate.
    #[must_use]
    pub fn new(hash: impl Into<String>) -> Self {
        Self {
            hash_algorithm: TimestampHashAlgorithm::Sha256,
            message_imprint_hash: hash.into(),
            nonce: None,
            cert_req: true,
        }
    }

    /// Create a timestamp request with a specific hash algorithm.
    #[must_use]
    pub fn with_algorithm(hash: impl Into<String>, algorithm: TimestampHashAlgorithm) -> Self {
        Self {
            hash_algorithm: algorithm,
            message_imprint_hash: hash.into(),
            nonce: None,
            cert_req: true,
        }
    }

    /// Add a random 16-byte nonce for replay protection.
    #[must_use]
    pub fn with_nonce(mut self) -> Self {
        // Use two UUIDs (32 bytes of v4 randomness) as the nonce source.
        // UUID v4 provides 122 bits of randomness; we only need 128 bits
        // for a 16-byte nonce, so two UUIDs provide ample entropy.
        let u1 = uuid::Uuid::new_v4();
        let u2 = uuid::Uuid::new_v4();
        let mut nonce = [0u8; 16];
        nonce[..8].copy_from_slice(&u1.as_bytes()[..8]);
        nonce[8..].copy_from_slice(&u2.as_bytes()[..8]);
        self.nonce = Some(hex_encode_slice(&nonce));
        self
    }

    /// Set whether to request the TSA's signing certificate.
    #[must_use]
    pub fn with_cert_req(mut self, cert_req: bool) -> Self {
        self.cert_req = cert_req;
        self
    }

    /// Create a request from a Merkle tree's root hash.
    #[must_use]
    pub fn from_merkle_root(tree: &MerkleTree) -> Self {
        Self::new(tree.root())
    }

    /// Create a request from an audit entry's hash.
    #[must_use]
    pub fn from_entry(entry: &AuditEntry) -> Self {
        Self::new(entry.hash())
    }

    /// Create a request from a chain's head hash.
    ///
    /// Returns `None` if the chain is empty.
    #[must_use]
    pub fn from_chain_head(chain: &AuditChain) -> Option<Self> {
        chain.head_hash().map(Self::new)
    }

    /// Encode this request as DER bytes (RFC 3161 Section 2.4.1).
    ///
    /// The returned bytes can be POSTed directly to a TSA endpoint
    /// with `Content-Type: application/timestamp-query`.
    ///
    /// # RFC 3161 TimeStampReq structure
    ///
    /// ```text
    /// TimeStampReq ::= SEQUENCE {
    ///   version          INTEGER { v1(1) },
    ///   messageImprint   MessageImprint,
    ///   nonce            INTEGER OPTIONAL,
    ///   certReq          BOOLEAN DEFAULT FALSE
    /// }
    /// MessageImprint ::= SEQUENCE {
    ///   hashAlgorithm    AlgorithmIdentifier,
    ///   hashedMessage    OCTET STRING
    /// }
    /// AlgorithmIdentifier ::= SEQUENCE {
    ///   algorithm        OBJECT IDENTIFIER,
    ///   parameters       ANY DEFINED BY algorithm OPTIONAL
    /// }
    /// ```
    #[must_use]
    pub fn to_der(&self) -> Vec<u8> {
        let hash_bytes = crate::hasher::hex_decode(&self.message_imprint_hash).unwrap_or_default();

        // AlgorithmIdentifier: SEQUENCE { OID, NULL }
        let oid_tlv = der_tlv(DER_OID, self.hash_algorithm.oid_bytes());
        let null_tlv = vec![0x05, 0x00]; // NULL
        let alg_id = der_sequence(&[&oid_tlv, &null_tlv]);

        // MessageImprint: SEQUENCE { AlgorithmIdentifier, OCTET STRING }
        let hash_tlv = der_tlv(DER_OCTET_STRING, &hash_bytes);
        let msg_imprint = der_sequence(&[&alg_id, &hash_tlv]);

        // version: INTEGER 1
        let version = der_tlv(DER_INTEGER, &[0x01]);

        // Build the full SEQUENCE contents
        let mut body_parts: Vec<&[u8]> = vec![&version, &msg_imprint];

        // nonce: INTEGER (optional)
        let nonce_tlv;
        if let Some(ref nonce_hex) = self.nonce {
            let nonce_bytes = crate::hasher::hex_decode(nonce_hex).unwrap_or_default();
            // DER INTEGER must not have leading zero unless needed for sign bit
            let trimmed = trim_leading_zeros(&nonce_bytes);
            // If high bit set, prepend 0x00 for positive integer
            let needs_pad = trimmed.first().is_some_and(|b| b & 0x80 != 0);
            if needs_pad {
                let mut padded = vec![0x00];
                padded.extend_from_slice(trimmed);
                nonce_tlv = der_tlv(DER_INTEGER, &padded);
            } else {
                nonce_tlv = der_tlv(DER_INTEGER, trimmed);
            }
            body_parts.push(&nonce_tlv);
        }

        // certReq: BOOLEAN (only encode if true, since DEFAULT is FALSE)
        let cert_tlv;
        if self.cert_req {
            cert_tlv = der_tlv(DER_BOOLEAN, &[0xFF]);
            body_parts.push(&cert_tlv);
        }

        der_sequence(&body_parts)
    }
}

// --- TimestampResponse ---

impl TimestampResponse {
    /// Parse a DER-encoded RFC 3161 `TimeStampResp`.
    ///
    /// Extracts the status and token from the TSA's response.
    ///
    /// # RFC 3161 TimeStampResp structure
    ///
    /// ```text
    /// TimeStampResp ::= SEQUENCE {
    ///   status          PKIStatusInfo,
    ///   timeStampToken  ContentInfo OPTIONAL
    /// }
    /// PKIStatusInfo ::= SEQUENCE {
    ///   status        PKIStatus,
    ///   statusString  PKIFreeText OPTIONAL,
    ///   failInfo      PKIFailureInfo OPTIONAL
    /// }
    /// PKIStatus ::= INTEGER
    /// ```
    pub fn from_der(der: &[u8]) -> crate::Result<Self> {
        // Parse outer SEQUENCE
        let (_, outer_content) =
            der_parse_tlv(der).map_err(|e| LibroError::Der(format!("outer sequence: {e}")))?;

        if der.first() != Some(&DER_SEQUENCE) {
            return Err(LibroError::Der("expected SEQUENCE at top level".into()));
        }

        // Parse PKIStatusInfo SEQUENCE
        let (status_info_len, status_info_content) = der_parse_tlv(outer_content)
            .map_err(|e| LibroError::Der(format!("PKIStatusInfo: {e}")))?;

        if outer_content.first() != Some(&DER_SEQUENCE) {
            return Err(LibroError::Der(
                "expected SEQUENCE for PKIStatusInfo".into(),
            ));
        }

        // Parse PKIStatus INTEGER from within status info
        let (_, status_value_bytes) = der_parse_tlv(status_info_content)
            .map_err(|e| LibroError::Der(format!("PKIStatus integer: {e}")))?;

        if status_info_content.first() != Some(&DER_INTEGER) {
            return Err(LibroError::Der("expected INTEGER for PKIStatus".into()));
        }

        let status_value = der_decode_unsigned(status_value_bytes);
        let status = TimestampStatus::from_value(status_value)
            .ok_or_else(|| LibroError::Der(format!("unknown PKI status: {status_value}")))?;

        // The token is the remainder after PKIStatusInfo
        let remaining = &outer_content[status_info_len..];
        let token = if remaining.is_empty() {
            None
        } else {
            Some(hex_encode_slice(remaining))
        };

        Ok(Self {
            status,
            token,
            status_string: None,
        })
    }

    /// Whether the timestamp was granted.
    #[inline]
    #[must_use]
    pub fn is_granted(&self) -> bool {
        matches!(
            self.status,
            TimestampStatus::Granted | TimestampStatus::GrantedWithMods
        )
    }
}

// --- TimestampAttestation ---

impl TimestampAttestation {
    /// Create an attestation from a hash, subject, algorithm, and response.
    ///
    /// Returns an error if the response was not granted.
    pub fn new(
        hash: impl Into<String>,
        subject: TimestampSubject,
        algorithm: TimestampHashAlgorithm,
        response: &TimestampResponse,
    ) -> crate::Result<Self> {
        if !response.is_granted() {
            return Err(LibroError::Timestamp(format!(
                "TSA did not grant timestamp: {:?}",
                response.status
            )));
        }
        let token_der = response
            .token
            .as_deref()
            .ok_or_else(|| LibroError::Timestamp("granted response has no token".into()))?;

        Ok(Self {
            hash: hash.into(),
            hash_subject: subject,
            hash_algorithm: algorithm,
            token_der: token_der.to_owned(),
            tsa_url: None,
            created_at: Utc::now(),
        })
    }

    /// Create an attestation for a Merkle root.
    pub fn for_merkle_root(tree: &MerkleTree, response: &TimestampResponse) -> crate::Result<Self> {
        Self::new(
            tree.root(),
            TimestampSubject::MerkleRoot,
            TimestampHashAlgorithm::Sha256,
            response,
        )
    }

    /// Create an attestation for an entry hash.
    pub fn for_entry(entry: &AuditEntry, response: &TimestampResponse) -> crate::Result<Self> {
        Self::new(
            entry.hash(),
            TimestampSubject::EntryHash,
            TimestampHashAlgorithm::Sha256,
            response,
        )
    }

    /// Set the TSA URL (informational, for re-verification).
    #[must_use]
    pub fn with_tsa_url(mut self, url: impl Into<String>) -> Self {
        self.tsa_url = Some(url.into());
        self
    }

    /// Verify that this attestation's hash matches the given hash.
    ///
    /// This is a local check — it does NOT verify the TSA's signature.
    /// Full TSA signature verification requires the TSA's certificate chain,
    /// which is the consumer's responsibility.
    #[must_use]
    pub fn verify_hash(&self, hash: &str) -> bool {
        constant_time_eq(&self.hash, hash)
    }
}

// --- DER helpers (private) ---

/// Encode a DER TLV (tag-length-value).
fn der_tlv(tag: u8, value: &[u8]) -> Vec<u8> {
    let mut out = vec![tag];
    out.extend_from_slice(&der_length(value.len()));
    out.extend_from_slice(value);
    out
}

/// Encode a DER SEQUENCE from pre-encoded parts.
fn der_sequence(parts: &[&[u8]]) -> Vec<u8> {
    let total_len: usize = parts.iter().map(|p| p.len()).sum();
    let mut content = Vec::with_capacity(total_len);
    for part in parts {
        content.extend_from_slice(part);
    }
    der_tlv(DER_SEQUENCE, &content)
}

/// Encode DER length (supports definite short and long forms).
fn der_length(len: usize) -> Vec<u8> {
    if len < 128 {
        vec![len as u8]
    } else if len < 256 {
        vec![0x81, len as u8]
    } else if len < 65536 {
        vec![0x82, (len >> 8) as u8, len as u8]
    } else {
        // 3-byte length for very large values
        vec![0x83, (len >> 16) as u8, (len >> 8) as u8, len as u8]
    }
}

/// Parse a DER TLV, returning (total consumed bytes, value slice).
fn der_parse_tlv(data: &[u8]) -> Result<(usize, &[u8]), &'static str> {
    if data.len() < 2 {
        return Err("truncated TLV");
    }

    let (len, header_size) = if data[1] < 128 {
        (data[1] as usize, 2)
    } else {
        let num_bytes = (data[1] & 0x7F) as usize;
        if data.len() < 2 + num_bytes {
            return Err("truncated length");
        }
        let mut len: usize = 0;
        for i in 0..num_bytes {
            len = (len << 8) | data[2 + i] as usize;
        }
        (len, 2 + num_bytes)
    };

    let total = header_size + len;
    if data.len() < total {
        return Err("truncated value");
    }

    Ok((total, &data[header_size..total]))
}

/// Decode an unsigned integer from DER INTEGER bytes.
fn der_decode_unsigned(bytes: &[u8]) -> u64 {
    let mut value: u64 = 0;
    for &b in bytes {
        value = (value << 8) | b as u64;
    }
    value
}

/// Trim leading zeros from a byte slice (for DER INTEGER encoding).
fn trim_leading_zeros(bytes: &[u8]) -> &[u8] {
    let start = bytes.iter().position(|&b| b != 0).unwrap_or(bytes.len());
    if start == bytes.len() {
        // All zeros — return a single zero
        &[0]
    } else {
        &bytes[start..]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entry::EventSeverity;

    #[test]
    fn request_new() {
        let req = TimestampRequest::new("abcdef1234");
        assert_eq!(req.message_imprint_hash, "abcdef1234");
        assert_eq!(req.hash_algorithm, TimestampHashAlgorithm::Sha256);
        assert!(req.cert_req);
        assert!(req.nonce.is_none());
    }

    #[test]
    fn request_with_algorithm() {
        let req = TimestampRequest::with_algorithm("abc", TimestampHashAlgorithm::Sha512);
        assert_eq!(req.hash_algorithm, TimestampHashAlgorithm::Sha512);
    }

    #[test]
    fn request_with_nonce() {
        let req = TimestampRequest::new("abc").with_nonce();
        let nonce = req.nonce.as_ref().unwrap();
        assert_eq!(nonce.len(), 32); // 16 bytes = 32 hex chars
    }

    #[test]
    fn request_nonce_varies() {
        let r1 = TimestampRequest::new("abc").with_nonce();
        let r2 = TimestampRequest::new("abc").with_nonce();
        // Different UUIDs as entropy → different nonces
        assert_ne!(r1.nonce, r2.nonce);
    }

    #[test]
    fn request_from_entry() {
        let entry = AuditEntry::new(EventSeverity::Info, "s", "a", serde_json::json!({}), "");
        let req = TimestampRequest::from_entry(&entry);
        assert_eq!(req.message_imprint_hash, entry.hash());
    }

    #[test]
    fn request_from_merkle_root() {
        let e1 = AuditEntry::new(EventSeverity::Info, "s", "a", serde_json::json!({}), "");
        let tree = MerkleTree::build(&[e1]).unwrap();
        let req = TimestampRequest::from_merkle_root(&tree);
        assert_eq!(req.message_imprint_hash, tree.root());
    }

    #[test]
    fn request_from_chain_head() {
        let mut chain = AuditChain::new();
        assert!(TimestampRequest::from_chain_head(&chain).is_none());

        chain.append(EventSeverity::Info, "s", "a", serde_json::json!({}));
        let req = TimestampRequest::from_chain_head(&chain).unwrap();
        assert_eq!(req.message_imprint_hash, chain.head_hash().unwrap());
    }

    #[test]
    fn request_to_der_structure() {
        // Use a known 32-byte hash (SHA-256 output)
        let hash_hex = "a".repeat(64); // 32 bytes
        let req = TimestampRequest::new(&hash_hex);
        let der = req.to_der();

        // Should start with SEQUENCE tag
        assert_eq!(der[0], DER_SEQUENCE);

        // Should be parseable
        let (_, content) = der_parse_tlv(&der).unwrap();

        // First element: version INTEGER 1
        assert_eq!(content[0], DER_INTEGER);
        let (ver_len, ver_val) = der_parse_tlv(content).unwrap();
        assert_eq!(ver_val, &[0x01]);

        // Second element: MessageImprint SEQUENCE
        assert_eq!(content[ver_len], DER_SEQUENCE);
    }

    #[test]
    fn request_to_der_with_nonce() {
        let hash_hex = "b".repeat(64);
        let req = TimestampRequest::new(&hash_hex).with_nonce();
        let der = req.to_der();

        // Should be longer than without nonce
        let der_no_nonce = TimestampRequest::new(&hash_hex).to_der();
        assert!(der.len() > der_no_nonce.len());
    }

    #[test]
    fn request_to_der_no_cert_req() {
        let hash_hex = "c".repeat(64);
        let req = TimestampRequest::new(&hash_hex).with_cert_req(false);
        let der = req.to_der();

        // Should be shorter without certReq BOOLEAN
        let der_with_cert = TimestampRequest::new(&hash_hex).to_der();
        assert!(der.len() < der_with_cert.len());
    }

    #[test]
    fn response_from_der_granted() {
        // Build a synthetic TSA response: SEQUENCE { PKIStatusInfo { status=0 }, token }
        let status_int = der_tlv(DER_INTEGER, &[0x00]); // status = 0 (granted)
        let status_info = der_sequence(&[&status_int]);
        let fake_token = vec![0x30, 0x03, 0x01, 0x01, 0xFF]; // fake SEQUENCE
        let mut resp_content = Vec::new();
        resp_content.extend_from_slice(&status_info);
        resp_content.extend_from_slice(&fake_token);
        let resp_der = der_tlv(DER_SEQUENCE, &resp_content);

        let resp = TimestampResponse::from_der(&resp_der).unwrap();
        assert_eq!(resp.status, TimestampStatus::Granted);
        assert!(resp.is_granted());
        assert!(resp.token.is_some());
    }

    #[test]
    fn response_from_der_rejected() {
        let status_int = der_tlv(DER_INTEGER, &[0x02]); // status = 2 (rejection)
        let status_info = der_sequence(&[&status_int]);
        let resp_der = der_sequence(&[&status_info]);

        let resp = TimestampResponse::from_der(&resp_der).unwrap();
        assert_eq!(resp.status, TimestampStatus::Rejection);
        assert!(!resp.is_granted());
        assert!(resp.token.is_none());
    }

    #[test]
    fn response_from_der_invalid() {
        assert!(TimestampResponse::from_der(&[]).is_err());
        assert!(TimestampResponse::from_der(&[0x01, 0x02]).is_err());
    }

    #[test]
    fn attestation_granted() {
        let resp = TimestampResponse {
            status: TimestampStatus::Granted,
            token: Some("deadbeef".into()),
            status_string: None,
        };
        let att = TimestampAttestation::new(
            "myhash",
            TimestampSubject::EntryHash,
            TimestampHashAlgorithm::Sha256,
            &resp,
        )
        .unwrap();
        assert_eq!(att.hash, "myhash");
        assert_eq!(att.token_der, "deadbeef");
        assert!(att.verify_hash("myhash"));
        assert!(!att.verify_hash("other"));
    }

    #[test]
    fn attestation_rejected() {
        let resp = TimestampResponse {
            status: TimestampStatus::Rejection,
            token: None,
            status_string: None,
        };
        let err = TimestampAttestation::new(
            "myhash",
            TimestampSubject::EntryHash,
            TimestampHashAlgorithm::Sha256,
            &resp,
        )
        .unwrap_err();
        assert!(err.to_string().contains("Rejection"));
    }

    #[test]
    fn attestation_for_merkle_root() {
        let e1 = AuditEntry::new(EventSeverity::Info, "s", "a", serde_json::json!({}), "");
        let tree = MerkleTree::build(&[e1]).unwrap();
        let resp = TimestampResponse {
            status: TimestampStatus::Granted,
            token: Some("token123".into()),
            status_string: None,
        };
        let att = TimestampAttestation::for_merkle_root(&tree, &resp).unwrap();
        assert_eq!(att.hash, tree.root());
        assert_eq!(att.hash_subject, TimestampSubject::MerkleRoot);
    }

    #[test]
    fn attestation_for_entry() {
        let entry = AuditEntry::new(EventSeverity::Info, "s", "a", serde_json::json!({}), "");
        let resp = TimestampResponse {
            status: TimestampStatus::Granted,
            token: Some("token123".into()),
            status_string: None,
        };
        let att = TimestampAttestation::for_entry(&entry, &resp).unwrap();
        assert_eq!(att.hash, entry.hash());
        assert_eq!(att.hash_subject, TimestampSubject::EntryHash);
    }

    #[test]
    fn attestation_with_tsa_url() {
        let resp = TimestampResponse {
            status: TimestampStatus::Granted,
            token: Some("tok".into()),
            status_string: None,
        };
        let att = TimestampAttestation::new(
            "h",
            TimestampSubject::ChainHead,
            TimestampHashAlgorithm::Sha256,
            &resp,
        )
        .unwrap()
        .with_tsa_url("https://tsa.example.com");
        assert_eq!(att.tsa_url.as_deref(), Some("https://tsa.example.com"));
    }

    #[test]
    fn serde_roundtrip_request() {
        let req = TimestampRequest::new("abc123").with_nonce();
        let json = serde_json::to_string(&req).unwrap();
        let back: TimestampRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(req, back);
    }

    #[test]
    fn serde_roundtrip_response() {
        let resp = TimestampResponse {
            status: TimestampStatus::GrantedWithMods,
            token: Some("aabb".into()),
            status_string: Some("ok".into()),
        };
        let json = serde_json::to_string(&resp).unwrap();
        let back: TimestampResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(resp, back);
    }

    #[test]
    fn serde_roundtrip_attestation() {
        let resp = TimestampResponse {
            status: TimestampStatus::Granted,
            token: Some("tok".into()),
            status_string: None,
        };
        let att = TimestampAttestation::new(
            "hash",
            TimestampSubject::MerkleRoot,
            TimestampHashAlgorithm::Sha256,
            &resp,
        )
        .unwrap();
        let json = serde_json::to_string(&att).unwrap();
        let back: TimestampAttestation = serde_json::from_str(&json).unwrap();
        assert_eq!(att, back);
    }

    #[test]
    fn all_status_variants() {
        for v in 0..=5 {
            assert!(TimestampStatus::from_value(v).is_some());
        }
        assert!(TimestampStatus::from_value(6).is_none());
    }

    #[test]
    fn der_length_encoding() {
        assert_eq!(der_length(0), vec![0x00]);
        assert_eq!(der_length(127), vec![0x7F]);
        assert_eq!(der_length(128), vec![0x81, 0x80]);
        assert_eq!(der_length(255), vec![0x81, 0xFF]);
        assert_eq!(der_length(256), vec![0x82, 0x01, 0x00]);
    }

    #[test]
    fn der_roundtrip_tlv() {
        let data = b"hello world";
        let encoded = der_tlv(DER_OCTET_STRING, data);
        let (total, decoded) = der_parse_tlv(&encoded).unwrap();
        assert_eq!(total, encoded.len());
        assert_eq!(decoded, data);
    }
}
