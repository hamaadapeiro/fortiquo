use crate::Address;
use serde::{Deserialize, Serialize};

/// Identifies the cryptographic algorithm used (ML-DSA-44, etc).
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum AlgorithmId {
    MlDsa44 = 1,
}

/// A public key bytes wrapper.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PublicKeyBytes(pub Vec<u8>);

impl PublicKeyBytes {
    /// Create from a vector.
    pub fn new(bytes: Vec<u8>) -> Self {
        PublicKeyBytes(bytes)
    }

    /// Get as slice.
    pub fn as_slice(&self) -> &[u8] {
        &self.0
    }

    /// Get length.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Check if empty.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

/// A signature bytes wrapper.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SignatureBytes(pub Vec<u8>);

impl SignatureBytes {
    /// Create from a vector.
    pub fn new(bytes: Vec<u8>) -> Self {
        SignatureBytes(bytes)
    }

    /// Get as slice.
    pub fn as_slice(&self) -> &[u8] {
        &self.0
    }

    /// Get length.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Check if empty.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

/// The kind of transaction.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransactionKind {
    /// Simple value transfer (no data).
    Transfer,
    /// Call to a smart contract.
    ContractCall,
    /// Create a new smart contract.
    ContractCreate,
}

/// An unsigned transaction (not yet signed).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UnsignedTransaction {
    /// Chain ID for replay protection.
    pub chain_id: u64,
    /// Account nonce (incrementing sequence number).
    pub nonce: u64,
    /// Maximum gas allowed for execution.
    pub gas_limit: u64,
    /// Maximum fee per unit of gas.
    pub max_fee_per_gas: u128,
    /// Priority fee per unit of gas (for MEV).
    pub priority_fee_per_gas: u128,
    /// Target address (None for contract create).
    pub to: Option<Address>,
    /// Value to transfer in wei.
    pub value: u128,
    /// Transaction data (contract code for creation, calldata for call).
    pub data: Vec<u8>,
    /// Type of transaction.
    pub tx_kind: TransactionKind,
    /// Optional memo or additional info.
    pub memo: Option<String>,
}

impl UnsignedTransaction {
    /// Serialize for signing (canonical format).
    pub fn serialize_for_signing(&self) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        Ok(postcard::to_allocvec(self)?)
    }

    /// Compute the transaction hash.
    pub fn hash(&self) -> crate::TxHash {
        let bytes = self.serialize_for_signing().expect("serialization failed");
        let hash = crate::Hash::compute(&bytes);
        crate::TxHash::new(hash)
    }
}

/// A signed transaction (with signature and public key).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SignedTransaction {
    /// The unsigned transaction.
    pub unsigned_tx: UnsignedTransaction,
    /// Sender's public key (ML-DSA-44).
    pub public_key: PublicKeyBytes,
    /// Signature over the unsigned transaction.
    pub signature: SignatureBytes,
    /// Algorithm ID (ML-DSA-44, etc).
    pub algorithm_id: AlgorithmId,
}

impl SignedTransaction {
    /// Create a new signed transaction.
    pub fn new(
        unsigned_tx: UnsignedTransaction,
        public_key: PublicKeyBytes,
        signature: SignatureBytes,
        algorithm_id: AlgorithmId,
    ) -> Self {
        SignedTransaction {
            unsigned_tx,
            public_key,
            signature,
            algorithm_id,
        }
    }

    /// Get the transaction hash.
    pub fn hash(&self) -> crate::TxHash {
        self.unsigned_tx.hash()
    }

    /// Serialize the signed transaction.
    pub fn serialize(&self) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        Ok(postcard::to_allocvec(self)?)
    }

    /// Deserialize a signed transaction.
    pub fn deserialize(data: &[u8]) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(postcard::from_bytes(data)?)
    }

    /// Get the bytes to verify signature over.
    pub fn signature_bytes(&self) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        self.unsigned_tx.serialize_for_signing()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transaction_kind() {
        let transfer = TransactionKind::Transfer;
        let call = TransactionKind::ContractCall;
        let create = TransactionKind::ContractCreate;
        assert_ne!(transfer, call);
        assert_ne!(call, create);
    }

    #[test]
    fn test_unsigned_transaction_serialization() {
        let tx = UnsignedTransaction {
            chain_id: 1,
            nonce: 0,
            gas_limit: 21_000,
            max_fee_per_gas: 1_000_000_000,
            priority_fee_per_gas: 100_000_000,
            to: Some(Address::new([0x01u8; 20])),
            value: 1_000_000_000_000_000_000, // 1 ether
            data: vec![],
            tx_kind: TransactionKind::Transfer,
            memo: None,
        };

        let serialized = tx.serialize_for_signing().unwrap();
        assert!(!serialized.is_empty());

        // Verify it's deterministic
        let serialized2 = tx.serialize_for_signing().unwrap();
        assert_eq!(serialized, serialized2);
    }

    #[test]
    fn test_unsigned_transaction_hash() {
        let tx = UnsignedTransaction {
            chain_id: 1,
            nonce: 5,
            gas_limit: 21_000,
            max_fee_per_gas: 1_000_000_000,
            priority_fee_per_gas: 0,
            to: Some(Address::new([0x02u8; 20])),
            value: 0,
            data: vec![],
            tx_kind: TransactionKind::Transfer,
            memo: None,
        };

        let hash1 = tx.hash();
        let hash2 = tx.hash();
        assert_eq!(hash1, hash2); // deterministic

        // Different tx gets different hash
        let mut tx2 = tx.clone();
        tx2.nonce = 6;
        let hash3 = tx2.hash();
        assert_ne!(hash1, hash3);
    }

    #[test]
    fn test_signed_transaction_roundtrip() {
        let unsigned = UnsignedTransaction {
            chain_id: 1,
            nonce: 0,
            gas_limit: 21_000,
            max_fee_per_gas: 1_000_000_000,
            priority_fee_per_gas: 0,
            to: None,
            value: 0,
            data: vec![1, 2, 3, 4],
            tx_kind: TransactionKind::ContractCreate,
            memo: Some("test".to_string()),
        };

        let signed = SignedTransaction::new(
            unsigned,
            PublicKeyBytes::new(vec![42; 1184]), // ML-DSA-44 pubkey size
            SignatureBytes::new(vec![99; 4668]), // ML-DSA-44 sig size
            AlgorithmId::MlDsa44,
        );

        let serialized = signed.serialize().unwrap();
        let deserialized = SignedTransaction::deserialize(&serialized).unwrap();

        assert_eq!(
            signed.unsigned_tx.chain_id,
            deserialized.unsigned_tx.chain_id
        );
        assert_eq!(signed.unsigned_tx.nonce, deserialized.unsigned_tx.nonce);
        assert_eq!(signed.public_key, deserialized.public_key);
        assert_eq!(signed.signature, deserialized.signature);
    }

    #[test]
    fn test_public_key_bytes() {
        let key = PublicKeyBytes::new(vec![1, 2, 3]);
        assert_eq!(key.len(), 3);
        assert!(!key.is_empty());
        assert_eq!(key.as_slice(), &[1, 2, 3]);
    }

    #[test]
    fn test_signature_bytes() {
        let sig = SignatureBytes::new(vec![4, 5, 6]);
        assert_eq!(sig.len(), 3);
        assert!(!sig.is_empty());
        assert_eq!(sig.as_slice(), &[4, 5, 6]);
    }
}
