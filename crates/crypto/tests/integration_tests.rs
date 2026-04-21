//! Integration tests for cryptographic operations

use fortiquo_crypto::{
    address_deriver::Blake3AddressDeriver, hasher::DomainSeparatedHasher,
    ml_dsa::MlDsa44Keypair, schemes::DefaultVerifier, AddressDeriver, Signer, Verifier,
};
use fortiquo_types::AlgorithmId;
use std::sync::Arc;

#[test]
fn test_full_sign_and_verify_flow() {
    let seed = b"integration test seed";
    let keypair = MlDsa44Keypair::from_seed(seed).unwrap();
    let message = b"test message for signing";

    // Sign the message
    let signature = keypair.sign(message).unwrap();
    assert_eq!(signature.len(), 4668);

    // Verify the signature
    let scheme = fortiquo_crypto::ml_dsa::MlDsa44Scheme;
    let verifier = DefaultVerifier::new(Arc::new(scheme));
    let public_key = keypair.public_key().unwrap();
    let result = verifier.verify(message, &signature, &public_key);
    assert!(result.is_ok());
}

#[test]
fn test_address_derivation_from_keypair() {
    let seed = b"address derivation test";
    let keypair = MlDsa44Keypair::from_seed(seed).unwrap();
    let address = keypair.address().unwrap();

    assert_eq!(address.as_bytes().len(), 20);
    println!("Derived address: {}", address);
}

#[test]
fn test_multiple_seeds_different_keypairs() {
    let seed1 = b"seed one";
    let seed2 = b"seed two";

    let kp1 = MlDsa44Keypair::from_seed(seed1).unwrap();
    let kp2 = MlDsa44Keypair::from_seed(seed2).unwrap();

    assert_ne!(kp1.public_key(), kp2.public_key());
}

#[test]
fn test_domain_separation_hashing() {
    use fortiquo_crypto::hasher::DomainSeparation;

    let data = b"important data";
    let tx_hash = DomainSeparatedHasher::new(DomainSeparation::Transaction).hash(data);
    let block_hash = DomainSeparatedHasher::new(DomainSeparation::Block).hash(data);
    let poh_hash = DomainSeparatedHasher::new(DomainSeparation::ProofOfHistory).hash(data);
    let receipt_hash = DomainSeparatedHasher::new(DomainSeparation::Receipt).hash(data);
    let account_hash = DomainSeparatedHasher::new(DomainSeparation::AccountState).hash(data);

    // All hashes should be different
    assert_ne!(tx_hash, block_hash);
    assert_ne!(tx_hash, poh_hash);
    assert_ne!(tx_hash, receipt_hash);
    assert_ne!(tx_hash, account_hash);
}

#[test]
fn test_algorithm_id_matching() {
    let keypair = MlDsa44Keypair::from_seed(b"algo test").unwrap();
    let scheme = fortiquo_crypto::ml_dsa::MlDsa44Scheme;

    assert_eq!(
        keypair.algorithm_id(),
        scheme.algorithm_id(),
        "Algorithm IDs should match"
    );
    assert_eq!(keypair.algorithm_id(), AlgorithmId::MlDsa44);
}

#[test]
fn test_key_size_constants() {
    let scheme = fortiquo_crypto::ml_dsa::MlDsa44Scheme;
    assert_eq!(scheme.expected_public_key_size(), 1184);
    assert_eq!(scheme.expected_signature_size(), 4668);
}

#[test]
fn test_deterministic_signature_generation() {
    let seed = b"deterministic test";
    let keypair = MlDsa44Keypair::from_seed(seed).unwrap();
    let message = b"sign this message";

    let sig1 = keypair.sign(message).unwrap();
    let sig2 = keypair.sign(message).unwrap();
    let sig3 = keypair.sign(message).unwrap();

    assert_eq!(sig1, sig2, "Signatures should be deterministic");
    assert_eq!(sig2, sig3, "Signatures should be deterministic");
}

#[test]
fn test_signature_tampering_detection() {
    let seed = b"tampering test";
    let keypair = MlDsa44Keypair::from_seed(seed).unwrap();
    let message = b"tamper this";
    let mut signature = keypair.sign(message).unwrap();

    // Tamper with the signature
    signature.0[0] ^= 0xFF;
    signature.0[100] ^= 0xAA;

    // Verification should fail
    let scheme = fortiquo_crypto::ml_dsa::MlDsa44Scheme;
    let valid = scheme
        .verify(message, &signature, keypair.public_key())
        .unwrap();
    assert!(!valid, "Tampered signature should not verify");
}

#[test]
fn test_seed_to_address_reproducibility() {
    let seed = b"reproducibility test";

    // Generate multiple times from the same seed
    let addr1 = MlDsa44Keypair::from_seed(seed)
        .unwrap()
        .address()
        .unwrap();
    let addr2 = MlDsa44Keypair::from_seed(seed)
        .unwrap()
        .address()
        .unwrap();
    let addr3 = MlDsa44Keypair::from_seed(seed)
        .unwrap()
        .address()
        .unwrap();

    assert_eq!(addr1, addr2, "Address generation should be reproducible");
    assert_eq!(addr2, addr3, "Address generation should be reproducible");
}
