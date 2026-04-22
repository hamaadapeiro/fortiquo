//! Cryptographic primitives and traits for Fortiquo.
//!
//! This crate provides trait-based abstractions for:
//! - ML-DSA-44 key generation, signing, and verification
//! - Address derivation from public keys
//! - Hashing with domain separation
//!
//! All crypto is swappable behind traits, allowing for testing and future algorithm upgrades.

pub mod error;
pub mod schemes;
pub mod address_deriver;
pub mod hasher;
pub mod ml_dsa;

pub use error::CryptoError;
pub use schemes::{PublicKeyScheme, Signer, Verifier};
pub use address_deriver::AddressDeriver;
pub use hasher::DomainSeparatedHasher;
pub use ml_dsa::{MlDsa44Keypair, DEFAULT_ADDRESS_DERIVER};
