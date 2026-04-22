use serde::{Deserialize, Serialize};
use std::fmt;

/// A 20-byte address, derived from ML-DSA public key.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Address([u8; 20]);

impl Address {
    /// Create a new address from a 20-byte array.
    pub fn new(bytes: [u8; 20]) -> Self {
        Address(bytes)
    }

    /// Get the address as a byte array.
    pub fn as_bytes(&self) -> &[u8; 20] {
        &self.0
    }

    /// Create an address from a byte slice. Returns None if not 20 bytes.
    pub fn try_from_slice(bytes: &[u8]) -> Option<Self> {
        if bytes.len() == 20 {
            let mut arr = [0u8; 20];
            arr.copy_from_slice(bytes);
            Some(Address(arr))
        } else {
            None
        }
    }
}

impl fmt::Display for Address {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "0x{}", hex::encode(self.0))
    }
}

impl AsRef<[u8]> for Address {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_address_creation() {
        let bytes = [1u8; 20];
        let addr = Address::new(bytes);
        assert_eq!(addr.as_bytes(), &bytes);
    }

    #[test]
    fn test_address_display() {
        let bytes = [0x12u8; 20];
        let addr = Address::new(bytes);
        let formatted = format!("{}", addr);
        assert!(formatted.starts_with("0x"));
        assert_eq!(formatted.len(), 42); // 0x + 40 hex chars
    }

    #[test]
    fn test_address_from_slice() {
        let bytes = [5u8; 20];
        let addr = Address::try_from_slice(&bytes).unwrap();
        assert_eq!(addr.as_bytes(), &bytes);

        let short = [0u8; 19];
        assert!(Address::try_from_slice(&short).is_none());
    }

    #[test]
    fn test_address_serialization() {
        let addr = Address::new([42u8; 20]);
        let json = serde_json::to_string(&addr).unwrap();
        let parsed: Address = serde_json::from_str(&json).unwrap();
        assert_eq!(addr, parsed);
    }
}
