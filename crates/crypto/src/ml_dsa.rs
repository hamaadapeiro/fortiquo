use crate::error::CryptoError;
use crate::address_deriver::{Blake3AddressDeriver, AddressDeriver};
use crate::schemes::{PublicKeyScheme, Signer};
use fortiquo_types::{Address, AlgorithmId, PublicKeyBytes, SignatureBytes};

/// Default address deriver for ML-DSA-44.
pub static DEFAULT_ADDRESS_DERIVER: Blake3AddressDeriver = Blake3AddressDeriver;

/// ML-DSA-44 keypair (deterministic test implementation for MVP).
///
/// ⚠️  WARNING: This is a test implementation. NOT FOR PRODUCTION.
/// For production, integrate with NIST-approved ML-DSA crate (FIPS 204).
///
/// The test implementation uses deterministic generation from a seed for reproducibility.
#[derive(Clone)]
pub struct MlDsa44Keypair {
    seed: Vec<u8>,
    public_key: PublicKeyBytes,
    private_key: Vec<u8>,
}

impl MlDsa44Keypair {
    /// Create a new ML-DSA-44 keypair from a seed (test implementation).
    ///
    /// ⚠️  NOT FOR PRODUCTION. Test only.
    pub fn from_seed(seed: &[u8]) -> Result<Self, CryptoError> {
        if seed.len() < 32 {
            return Err(CryptoError::InvalidPrivateKey);
        }

        // Test implementation: hash seed to generate keypair
        let hasher = blake3::hash(seed);
        let hash_bytes = hasher.as_bytes();

        // Simulate ML-DSA-44 keypair generation
        // Public key: first 1184 bytes (repeated pattern from hash)
        let mut public_key_bytes = vec![0u8; 1184];
        for i in 0..1184 {
            public_key_bytes[i] = hash_bytes[(i % 32)] ^ (i as u8);
        }

        // Private key: next 2544 bytes (ML-DSA-44 private key size)
        let mut private_key = vec![0u8; 2544];
        let seed_hash2 = blake3::hash(&[&seed, &hash_bytes].concat());
        for i in 0..2544 {
            private_key[i] = seed_hash2.as_bytes()[(i % 32)] ^ ((i >> 8) as u8);
        }

        Ok(MlDsa44Keypair {
            seed: seed.to_vec(),
            public_key: PublicKeyBytes::new(public_key_bytes),
            private_key,
        })
    }

    /// Get the public key.
    pub fn public_key(&self) -> &PublicKeyBytes {
        &self.public_key
    }

    /// Get the seed (for testing).
    pub fn seed(&self) -> &[u8] {
        &self.seed
    }

    /// Derive an address from this keypair.
    pub fn address(&self) -> Result<Address, CryptoError> {
        DEFAULT_ADDRESS_DERIVER.derive_address(&self.public_key)
    }
}

impl Signer for MlDsa44Keypair {
    fn sign(&self, message: &[u8]) -> Result<SignatureBytes, CryptoError> {
        // Test implementation: deterministic signature from message + private key
        let mut hasher = blake3::Hasher::new();
        hasher.update(message);
        hasher.update(&self.private_key);
        let sig_hash = hasher.finalize();

        // Generate 4668-byte signature
        let mut signature = vec![0u8; 4668];
        for i in 0..4668 {
            signature[i] = sig_hash.as_bytes()[(i % 32)] ^ (i as u8);
        }

        Ok(SignatureBytes::new(signature))
    }

    fn public_key(&self) -> Result<PublicKeyBytes, CryptoError> {
        Ok(self.public_key.clone())
    }

    fn algorithm_id(&self) -> AlgorithmId {
        AlgorithmId::MlDsa44
    }
}

/// ML-DSA-44 signature scheme (test implementation).
pub struct MlDsa44Scheme;

impl PublicKeyScheme for MlDsa44Scheme {
    fn verify(&self, message: &[u8], signature: &SignatureBytes, public_key: &PublicKeyBytes) -> Result<bool, CryptoError> {
        // Test implementation: recompute signature from message + public key hash
        if signature.len() != 4668 {
            return Err(CryptoError::InvalidSignature);
        }

        let mut hasher = blake3::Hasher::new();
        hasher.update(message);
        hasher.update(blake3::hash(public_key.as_slice()).as_bytes());
        let expected_sig_hash = hasher.finalize();

        // Verify signature matches expected pattern
        let mut expected_sig = vec![0u8; 4668];
        for i in 0..4668 {
            expected_sig[i] = expected_sig_hash.as_bytes()[(i % 32)] ^ (i as u8);
        }

        Ok(signature.as_slice() == expected_sig.as_slice())
    }

    fn algorithm_id(&self) -> AlgorithmId {
        AlgorithmId::MlDsa44
    }

    fn expected_public_key_size(&self) -> usize {
        1184
    }

    fn expected_signature_size(&self) -> usize {
        4668
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ml_dsa44_keypair_generation() {
        let seed = b"test seed for ml-dsa-44";
        let keypair = MlDsa44Keypair::from_seed(seed).unwrap();
        assert_eq!(keypair.public_key().len(), 1184);
    }

    #[test]
    fn test_ml_dsa44_deterministic() {
        let seed = b"same seed";
        let kp1 = MlDsa44Keypair::from_seed(seed).unwrap();
        let kp2 = MlDsa44Keypair::from_seed(seed).unwrap();

        assert_eq!(kp1.public_key(), kp2.public_key());
    }

    #[test]
    fn test_ml_dsa44_signing() {
        let seed = b"signing test seed";
        let keypair = MlDsa44Keypair::from_seed(seed).unwrap();

        let message = b"hello, blockchain";
        let sig1 = keypair.sign(message).unwrap();
        let sig2 = keypair.sign(message).unwrap();

        // Signature should be deterministic
        assert_eq!(sig1, sig2);
        assert_eq!(sig1.len(), 4668);
    }

    #[test]
    fn test_ml_dsa44_verification() {
        let seed = b"verification test";
        let keypair = MlDsa44Keypair::from_seed(seed).unwrap();
        let scheme = MlDsa44Scheme;

        let message = b"test message";
        let signature = keypair.sign(message).unwrap();

        let valid = scheme
            .verify(message, &signature, keypair.public_key())
            .unwrap();
        assert!(valid);
    }

    #[test]
    fn test_ml_dsa44_verification_fails_tampered_sig() {
        let seed = b"verification test";
        let keypair = MlDsa44Keypair::from_seed(seed).unwrap();
        let scheme = MlDsa44Scheme;

        let message = b"test message";
        let mut signature = keypair.sign(message).unwrap();

        // Tamper with signature
        signature.0[0] ^= 0xFF;

        let valid = scheme
            .verify(message, &signature, keypair.public_key())
            .unwrap();
        assert!(!valid);
    }

    #[test]
    fn test_ml_dsa44_verification_fails_wrong_message() {
        let seed = b"verification test";
        let keypair = MlDsa44Keypair::from_seed(seed).unwrap();
        let scheme = MlDsa44Scheme;

        let message = b"test message";
        let signature = keypair.sign(message).unwrap();

        let valid = scheme
            .verify(b"different message", &signature, keypair.public_key())
            .unwrap();
        assert!(!valid);
    }

    #[test]
    fn test_ml_dsa44_address_derivation() {
        let seed = b"address test";
        let keypair = MlDsa44Keypair::from_seed(seed).unwrap();
        let addr = keypair.address().unwrap();

        // Address should be deterministic
        let keypair2 = MlDsa44Keypair::from_seed(seed).unwrap();
        let addr2 = keypair2.address().unwrap();

        assert_eq!(addr, addr2);
    }

    #[test]
    fn test_ml_dsa44_scheme_properties() {
        let scheme = MlDsa44Scheme;
        assert_eq!(scheme.algorithm_id(), AlgorithmId::MlDsa44);
        assert_eq!(scheme.expected_public_key_size(), 1184);
        assert_eq!(scheme.expected_signature_size(), 4668);
    }
}
