use serde::{Deserialize, Serialize};

/// A validator identifier (32-byte hash).
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ValidatorId([u8; 32]);

impl ValidatorId {
    /// Create a new validator ID from a 32-byte array.
    pub fn new(bytes: [u8; 32]) -> Self {
        ValidatorId(bytes)
    }

    /// Get as byte array.
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// Get as slice.
    pub fn as_slice(&self) -> &[u8] {
        &self.0
    }
}

impl AsRef<[u8]> for ValidatorId {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl std::fmt::Display for ValidatorId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "0x{}", hex::encode(self.0))
    }
}

/// A validator in the set.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Validator {
    /// Validator ID.
    pub id: ValidatorId,
    /// Voting power or stake (unused in MVP PoA, reserved for PoS).
    pub stake: u64,
    /// Public key (for signature verification, future use).
    pub public_key: Option<Vec<u8>>,
    /// Commission or fee taken by validator (future use).
    pub commission: Option<u32>,
}

impl Validator {
    /// Create a new validator with ID and default values.
    pub fn new(id: ValidatorId) -> Self {
        Validator {
            id,
            stake: 1, // Default: equal voting power in MVP
            public_key: None,
            commission: None,
        }
    }

    /// Create with stake.
    pub fn with_stake(id: ValidatorId, stake: u64) -> Self {
        Validator {
            id,
            stake,
            public_key: None,
            commission: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validator_id_creation() {
        let bytes = [42u8; 32];
        let id = ValidatorId::new(bytes);
        assert_eq!(id.as_bytes(), &bytes);
    }

    #[test]
    fn test_validator_id_display() {
        let id = ValidatorId::new([0x12u8; 32]);
        let formatted = format!("{}", id);
        assert!(formatted.starts_with("0x"));
        assert_eq!(formatted.len(), 66); // 0x + 64 hex chars
    }

    #[test]
    fn test_validator_creation() {
        let id = ValidatorId::new([1u8; 32]);
        let validator = Validator::new(id);
        assert_eq!(validator.id, id);
        assert_eq!(validator.stake, 1);
        assert!(validator.public_key.is_none());
    }

    #[test]
    fn test_validator_with_stake() {
        let id = ValidatorId::new([2u8; 32]);
        let validator = Validator::with_stake(id, 1000);
        assert_eq!(validator.stake, 1000);
    }

    #[test]
    fn test_validator_serialization() {
        let id = ValidatorId::new([7u8; 32]);
        let validator = Validator::with_stake(id, 500);

        let serialized = serde_json::to_string(&validator).unwrap();
        let deserialized: Validator = serde_json::from_str(&serialized).unwrap();

        assert_eq!(validator.id, deserialized.id);
        assert_eq!(validator.stake, deserialized.stake);
    }
}
