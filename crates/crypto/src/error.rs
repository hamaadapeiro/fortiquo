use thiserror::Error;

/// Cryptographic errors.
#[derive(Error, Debug)]
pub enum CryptoError {
    #[error("Invalid signature")]
    InvalidSignature,

    #[error("Invalid public key")]
    InvalidPublicKey,

    #[error("Invalid private key")]
    InvalidPrivateKey,

    #[error("Signature verification failed")]
    VerificationFailed,

    #[error("Invalid address derivation")]
    InvalidAddressDerivation,

    #[error("Invalid algorithm: {0}")]
    InvalidAlgorithm(u8),

    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("Key generation failed")]
    KeyGenerationFailed,

    #[error("Signing failed")]
    SigningFailed,

    #[error("Hashing failed")]
    HashingFailed,
}
