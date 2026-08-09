//! Configuration hashing.

use core::fmt;

#[cfg(feature = "std")]
use sha2::{Digest, Sha256};

/// SHA-256 digest of canonical configuration bytes.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct ConfigHash(pub [u8; 32]);

impl ConfigHash {
    /// Parses a lowercase hex-encoded SHA-256 digest.
    #[cfg(feature = "std")]
    pub fn from_hex(input: &str) -> Option<Self> {
        let trimmed = input.trim();
        if trimmed.len() != 64 {
            return None;
        }
        let mut out = [0u8; 32];
        for (idx, chunk) in trimmed.as_bytes().chunks(2).enumerate() {
            if chunk.len() != 2 {
                return None;
            }
            let hi = hex_nibble(chunk[0])?;
            let lo = hex_nibble(chunk[1])?;
            out[idx] = (hi << 4) | lo;
        }
        Some(Self(out))
    }

    /// Computes the configuration hash from canonical bytes.
    #[cfg(feature = "std")]
    pub fn compute(canonical_bytes: &[u8]) -> Self {
        let digest = Sha256::digest(canonical_bytes);
        let mut out = [0u8; 32];
        out.copy_from_slice(&digest);
        Self(out)
    }

    /// Returns the digest bytes.
    #[must_use]
    pub const fn bytes(self) -> [u8; 32] {
        self.0
    }

    /// Encodes the digest as lowercase hex.
    #[cfg(feature = "std")]
    #[must_use]
    pub fn to_hex(self) -> alloc::string::String {
        let mut out = alloc::string::String::with_capacity(64);
        for byte in self.0 {
            out.push(hex_char(byte >> 4));
            out.push(hex_char(byte & 0xF));
        }
        out
    }
}

impl fmt::Debug for ConfigHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        #[cfg(feature = "std")]
        {
            write!(f, "ConfigHash({})", self.to_hex())
        }
        #[cfg(not(feature = "std"))]
        {
            write!(f, "ConfigHash([..; 32])")
        }
    }
}

#[cfg(feature = "std")]
const fn hex_nibble(ch: u8) -> Option<u8> {
    match ch {
        b'0'..=b'9' => Some(ch - b'0'),
        b'a'..=b'f' => Some(ch - b'a' + 10),
        _ => None,
    }
}

#[cfg(feature = "std")]
fn hex_char(nibble: u8) -> char {
    match nibble {
        0..=9 => char::from(b'0' + nibble),
        10..=15 => char::from(b'a' + (nibble - 10)),
        _ => '?',
    }
}
