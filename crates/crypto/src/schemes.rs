use crate::error::CryptoError;
use fortiquo_types::{PublicKeyBytes, SignatureBytes};

/// A cryptographic signature scheme.
pub trait PublicKeyScheme: Send + Sync {
    /// Verify a signature over a message.
    fn verify(&self, message: &[u8], signature: &SignatureBytes, public_key: &PublicKeyBytes) -> Result<bool, CryptoError>;

    /// Get the algorithm ID for this scheme.
    fn algorithm_id(&self) -> fortiquo_types::AlgorithmId;

    /// Expected public key size in bytes.
    fn expected_public_key_size(&self) -> usize;

    /// Expected signature size in bytes.
    fn expected_signature_size(&self) -> usize;
}

/// A signer interface.
pub trait Signer: Send + Sync {
    /// Sign a message and return the signature.
    fn sign(&self, message: &[u8]) -> Result<SignatureBytes, CryptoError>;

    /// Get the public key.
    fn public_key(&self) -> Result<PublicKeyBytes, CryptoError>;

    /// Get the algorithm ID.
    fn algorithm_id(&self) -> fortiquo_types::AlgorithmId;
}

/// A verifier interface.
pub trait Verifier: Send + Sync {
    /// Verify a signature.
    fn verify(&self, message: &[u8], signature: &SignatureBytes, public_key: &PublicKeyBytes) -> Result<(), CryptoError>;

    /// Get the algorithm ID this verifier supports.
    fn algorithm_id(&self) -> fortiquo_types::AlgorithmId;
}

/// Default implementation for verification using a scheme.
pub struct DefaultVerifier<S: PublicKeyScheme> {
    scheme: std::sync::Arc<S>,
}

impl<S: PublicKeyScheme> DefaultVerifier<S> {
    pub fn new(scheme: std::sync::Arc<S>) -> Self {
        DefaultVerifier { scheme }
    }
}

impl<S: PublicKeyScheme> Verifier for DefaultVerifier<S> {
    fn verify(&self, message: &[u8], signature: &SignatureBytes, public_key: &PublicKeyBytes) -> Result<(), CryptoError> {
        let valid = self.scheme.verify(message, signature, public_key)?;
        if valid {
            Ok(())
        } else {
            Err(CryptoError::VerificationFailed)
        }
    }

    fn algorithm_id(&self) -> fortiquo_types::AlgorithmId {
        self.scheme.algorithm_id()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_verifier_trait_exists() {
        // Compile-time check that trait is defined
        let _: Box<dyn Verifier> = Box::new(DefaultVerifier::new(std::sync::Arc::new(MockScheme)));
    }

    struct MockScheme;

    impl PublicKeyScheme for MockScheme {
        fn verify(&self, _: &[u8], _: &SignatureBytes, _: &PublicKeyBytes) -> Result<bool, CryptoError> {
            Ok(true)
        }

        fn algorithm_id(&self) -> fortiquo_types::AlgorithmId {
            fortiquo_types::AlgorithmId::MlDsa44
        }

        fn expected_public_key_size(&self) -> usize {
            1184
        }

        fn expected_signature_size(&self) -> usize {
            4668
        }
    }
}
