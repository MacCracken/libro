//! Internal hash abstraction.
//!
//! Defaults to BLAKE3 (4-10x faster than SHA-256, 256-bit output, 128-bit collision resistance).
//! Enable the `sha256` feature for NIST FIPS 180-4 compliance environments.

/// The hash algorithm identifier embedded in chain entries.
///
/// This enables verification across algorithm transitions — a verifier
/// can select the correct algorithm based on the stored identifier.
#[cfg(not(feature = "sha256"))]
pub(crate) const HASH_ALGORITHM: &str = "blake3";
#[cfg(feature = "sha256")]
pub(crate) const HASH_ALGORITHM: &str = "sha256";

/// Encode a fixed-size byte array as a lowercase hex string.
pub(crate) fn hex_encode<const N: usize>(bytes: [u8; N]) -> String {
    use std::fmt::Write;
    let mut hex = String::with_capacity(N * 2);
    for b in bytes {
        write!(hex, "{b:02x}").expect("write to String is infallible");
    }
    hex
}

/// Encode a byte slice as a lowercase hex string.
pub(crate) fn hex_encode_slice(bytes: &[u8]) -> String {
    use std::fmt::Write;
    let mut hex = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        write!(hex, "{b:02x}").expect("write to String is infallible");
    }
    hex
}

/// Decode a hex string into bytes. Returns `None` if the input is invalid hex.
pub(crate) fn hex_decode(hex: &str) -> Option<Vec<u8>> {
    if !hex.len().is_multiple_of(2) {
        return None;
    }
    (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).ok())
        .collect()
}

/// A streaming hasher that accumulates data and produces a hex-encoded digest.
pub(crate) struct ChainHasher {
    #[cfg(not(feature = "sha256"))]
    inner: blake3::Hasher,
    #[cfg(feature = "sha256")]
    inner: sha2::Sha256,
}

impl ChainHasher {
    /// Create a new hasher.
    #[inline]
    pub fn new() -> Self {
        Self {
            #[cfg(not(feature = "sha256"))]
            inner: blake3::Hasher::new(),
            #[cfg(feature = "sha256")]
            inner: <sha2::Sha256 as sha2::Digest>::new(),
        }
    }

    /// Feed data into the hasher.
    #[inline]
    pub fn update(&mut self, data: &[u8]) {
        #[cfg(not(feature = "sha256"))]
        {
            self.inner.update(data);
        }
        #[cfg(feature = "sha256")]
        {
            <sha2::Sha256 as sha2::Digest>::update(&mut self.inner, data);
        }
    }

    /// Finalize and return the hex-encoded digest.
    #[inline]
    pub fn finalize_hex(self) -> String {
        #[cfg(not(feature = "sha256"))]
        {
            self.inner.finalize().to_hex().to_string()
        }
        #[cfg(feature = "sha256")]
        {
            format!("{:x}", <sha2::Sha256 as sha2::Digest>::finalize(self.inner))
        }
    }
}
