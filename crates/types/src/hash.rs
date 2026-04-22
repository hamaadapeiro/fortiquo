use serde::{Deserialize, Serialize};
use std::fmt;

/// A 32-byte BLAKE3 hash.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Hash([u8; 32]);

impl Hash {
    /// Create a new hash from a 32-byte array.
    pub fn new(bytes: [u8; 32]) -> Self {
        Hash(bytes)
    }

    /// Get zero hash.
    pub fn zero() -> Self {
        Hash([0u8; 32])
    }

    /// Create from a byte slice. Returns None if not 32 bytes.
    pub fn try_from_slice(bytes: &[u8]) -> Option<Self> {
        if bytes.len() == 32 {
            let mut arr = [0u8; 32];
            arr.copy_from_slice(bytes);
            Some(Hash(arr))
        } else {
            None
        }
    }

    /// Get as byte array.
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// Compute hash of data using BLAKE3.
    pub fn compute(data: &[u8]) -> Self {
        let mut hasher = blake3::Hasher::new();
        hasher.update(data);
        let hash_out = hasher.finalize();
        let mut bytes = [0u8; 32];
        bytes.copy_from_slice(hash_out.as_bytes());
        Hash(bytes)
    }
}

impl fmt::Display for Hash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "0x{}", hex::encode(self.0))
    }
}

impl AsRef<[u8]> for Hash {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

/// A transaction hash (32-byte BLAKE3).
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct TxHash(pub Hash);

impl TxHash {
    /// Create a new transaction hash.
    pub fn new(hash: Hash) -> Self {
        TxHash(hash)
    }

    /// Get the underlying hash.
    pub fn as_hash(&self) -> Hash {
        self.0
    }

    /// Get as bytes.
    pub fn as_bytes(&self) -> &[u8; 32] {
        self.0.as_bytes()
    }
}

impl fmt::Display for TxHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A block hash (32-byte BLAKE3).
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct BlockHash(pub Hash);

impl BlockHash {
    /// Create a new block hash.
    pub fn new(hash: Hash) -> Self {
        BlockHash(hash)
    }

    /// Get the underlying hash.
    pub fn as_hash(&self) -> Hash {
        self.0
    }

    /// Get as bytes.
    pub fn as_bytes(&self) -> &[u8; 32] {
        self.0.as_bytes()
    }
}

impl fmt::Display for BlockHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_creation() {
        let bytes = [42u8; 32];
        let hash = Hash::new(bytes);
        assert_eq!(hash.as_bytes(), &bytes);
    }

    #[test]
    fn test_hash_compute() {
        let data = b"hello";
        let hash = Hash::compute(data);
        let hash2 = Hash::compute(data);
        assert_eq!(hash, hash2); // deterministic
    }

    #[test]
    fn test_hash_zero() {
        let zero = Hash::zero();
        assert_eq!(zero, Hash::new([0u8; 32]));
    }

    #[test]
    fn test_tx_hash_new() {
        let hash = Hash::new([7u8; 32]);
        let tx_hash = TxHash::new(hash);
        assert_eq!(tx_hash.as_hash(), hash);
    }

    #[test]
    fn test_block_hash_new() {
        let hash = Hash::new([13u8; 32]);
        let block_hash = BlockHash::new(hash);
        assert_eq!(block_hash.as_hash(), hash);
    }

    #[test]
    fn test_hash_serialization() {
        let hash = Hash::new([99u8; 32]);
        let json = serde_json::to_string(&hash).unwrap();
        let parsed: Hash = serde_json::from_str(&json).unwrap();
        assert_eq!(hash, parsed);
    }
}
