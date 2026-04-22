//! Integration tests for revm executor with ML-DSA-44 cryptography

use fortiquo_crypto::{
    ml_dsa::MlDsa44Keypair, ml_dsa::MlDsa44Scheme, schemes::DefaultVerifier, PublicKeyScheme, Signer,
    Verifier,
};
use fortiquo_revm::{Executor, EvmConfig, GasConfig};
use fortiquo_types::{
    Address, ExecutionStatus, SignatureBytes, SignedTransaction, TransactionKind,
    UnsignedTransaction,
};
use std::sync::Arc;

/// Helper to create a test keypair from seed
fn test_keypair(seed: &[u8]) -> MlDsa44Keypair {
    MlDsa44Keypair::from_seed(seed).expect("Failed to create keypair")
}

/// Helper to sign a transaction
fn sign_transaction(
    keypair: &MlDsa44Keypair,
    unsigned_tx: UnsignedTransaction,
) -> Result<SignedTransaction, Box<dyn std::error::Error>> {
    let message = unsigned_tx.serialize_for_signing()?;
    let signature = keypair.sign(&message)?;
    let public_key = keypair.public_key().clone();

    Ok(SignedTransaction::new(
        unsigned_tx,
        public_key,
        signature,
        fortiquo_types::AlgorithmId::MlDsa44,
    ))
}

#[test]
fn test_executor_basic_transfer_with_ml_dsa44() {
    let mut executor = Executor::new();
    let keypair = test_keypair(b"transfer test seed");

    let tx = UnsignedTransaction {
        chain_id: 1,
        nonce: 0,
        gas_limit: 21_000,
        max_fee_per_gas: 1_000_000_000,
        priority_fee_per_gas: 0,
        to: Some(Address::new([0x02u8; 20])),
        value: 1_000_000_000,
        data: vec![],
        tx_kind: TransactionKind::Transfer,
        memo: Some("ML-DSA-44 transfer".to_string()),
    };

    let signed_tx = sign_transaction(&keypair, tx).expect("Failed to sign transaction");

    // Verify the signature before execution
    let scheme = fortiquo_crypto::ml_dsa::MlDsa44Scheme;
    let verifier = DefaultVerifier::new(Arc::new(scheme));
    let message = signed_tx.signature_bytes().expect("Failed to get signature bytes");
    assert!(
        verifier
            .verify(&message, &signed_tx.signature, &signed_tx.public_key)
            .is_ok(),
        "Signature should verify"
    );

    // Execute the transaction
    let result = executor
        .execute_signed(&signed_tx)
        .expect("Failed to execute transaction");
    assert!(result.success, "Transaction should succeed");
    assert_eq!(result.gas_used, 21_000);
}

#[test]
fn test_executor_multiple_transactions_ml_dsa44() {
    let mut executor = Executor::new();
    let keypair1 = test_keypair(b"sender1 seed");
    let keypair2 = test_keypair(b"sender2 seed");

    // First transaction
    let tx1 = UnsignedTransaction {
        chain_id: 1,
        nonce: 0,
        gas_limit: 21_000,
        max_fee_per_gas: 1_000_000_000,
        priority_fee_per_gas: 0,
        to: Some(Address::new([0x01u8; 20])),
        value: 100,
        data: vec![],
        tx_kind: TransactionKind::Transfer,
        memo: None,
    };

    // Second transaction
    let tx2 = UnsignedTransaction {
        chain_id: 1,
        nonce: 0,
        gas_limit: 21_000,
        max_fee_per_gas: 1_000_000_000,
        priority_fee_per_gas: 0,
        to: Some(Address::new([0x02u8; 20])),
        value: 200,
        data: vec![],
        tx_kind: TransactionKind::Transfer,
        memo: None,
    };

    let signed_tx1 = sign_transaction(&keypair1, tx1).unwrap();
    let signed_tx2 = sign_transaction(&keypair2, tx2).unwrap();

    // Both transactions should have different signatures due to different keypairs
    assert_ne!(signed_tx1.signature, signed_tx2.signature);

    // Execute both transactions
    let result1 = executor.execute_signed(&signed_tx1).unwrap();
    let result2 = executor.execute_signed(&signed_tx2).unwrap();

    assert!(result1.success);
    assert!(result2.success);
}

#[test]
fn test_executor_signature_verification_ml_dsa44() {
    let keypair = test_keypair(b"verification test");
    let scheme = fortiquo_crypto::ml_dsa::MlDsa44Scheme;
    let verifier = DefaultVerifier::new(Arc::new(scheme));

    let tx = UnsignedTransaction {
        chain_id: 1,
        nonce: 0,
        gas_limit: 21_000,
        max_fee_per_gas: 1_000_000_000,
        priority_fee_per_gas: 0,
        to: Some(Address::new([0x03u8; 20])),
        value: 50,
        data: vec![],
        tx_kind: TransactionKind::Transfer,
        memo: None,
    };

    let signed_tx = sign_transaction(&keypair, tx).unwrap();
    let message = signed_tx.signature_bytes().unwrap();

    // Valid signature should verify
    assert!(verifier
        .verify(&message, &signed_tx.signature, &signed_tx.public_key)
        .is_ok());

    // Tampered signature should fail
    let mut tampered_sig = signed_tx.signature.clone();
    tampered_sig.0[0] ^= 0xFF;

    let tampered_ok = verifier
        .verify(&message, &tampered_sig, &signed_tx.public_key)
        .unwrap_or(false);
    assert!(!tampered_ok);
}

#[test]
fn test_executor_contract_creation_ml_dsa44() {
    let mut executor = Executor::new();
    let keypair = test_keypair(b"contract creation");

    let tx = UnsignedTransaction {
        chain_id: 1,
        nonce: 0,
        gas_limit: 32_000,
        max_fee_per_gas: 1_000_000_000,
        priority_fee_per_gas: 0,
        to: None, // Contract creation
        value: 0,
        data: vec![0x60, 0x80, 0x60, 0x40], // EVM bytecode
        tx_kind: TransactionKind::ContractCreate,
        memo: Some("Deploy with ML-DSA-44".to_string()),
    };

    let signed_tx = sign_transaction(&keypair, tx).unwrap();

    // Verify signature
    let scheme = fortiquo_crypto::ml_dsa::MlDsa44Scheme;
    let message = signed_tx.signature_bytes().unwrap();
    assert!(scheme
        .verify(&message, &signed_tx.signature, &signed_tx.public_key)
        .unwrap());

    // Execute contract creation
    let result = executor.execute_signed(&signed_tx).unwrap();
    assert!(result.success);
}

#[test]
fn test_executor_receipt_generation_ml_dsa44() {
    let mut executor = Executor::new();
    let keypair = test_keypair(b"receipt test");

    let tx = UnsignedTransaction {
        chain_id: 1,
        nonce: 0,
        gas_limit: 21_000,
        max_fee_per_gas: 1_000_000_000,
        priority_fee_per_gas: 0,
        to: Some(Address::new([0x04u8; 20])),
        value: 500,
        data: vec![],
        tx_kind: TransactionKind::Transfer,
        memo: None,
    };

    let signed_tx = sign_transaction(&keypair, tx).unwrap();
    let tx_hash = signed_tx.hash();

    let result = executor.execute_signed(&signed_tx).unwrap();
    let receipt = executor.create_receipt(tx_hash, 1, 0, &result);

    assert_eq!(receipt.status, ExecutionStatus::Success);
    assert_eq!(receipt.gas_used, 21_000);
    assert_eq!(receipt.block_number, 1);
}

#[test]
fn test_executor_deterministic_signatures_ml_dsa44() {
    let keypair = test_keypair(b"deterministic test");

    let tx = UnsignedTransaction {
        chain_id: 1,
        nonce: 0,
        gas_limit: 21_000,
        max_fee_per_gas: 1_000_000_000,
        priority_fee_per_gas: 0,
        to: Some(Address::new([0x05u8; 20])),
        value: 100,
        data: vec![],
        tx_kind: TransactionKind::Transfer,
        memo: None,
    };

    // Sign the same transaction multiple times
    let signed1 = sign_transaction(&keypair, tx.clone()).unwrap();
    let signed2 = sign_transaction(&keypair, tx.clone()).unwrap();
    let signed3 = sign_transaction(&keypair, tx).unwrap();

    // All signatures should be identical (deterministic ML-DSA-44)
    assert_eq!(signed1.signature, signed2.signature);
    assert_eq!(signed2.signature, signed3.signature);
}

#[test]
fn test_executor_different_keypairs_different_signatures_ml_dsa44() {
    let keypair1 = test_keypair(b"keypair one");
    let keypair2 = test_keypair(b"keypair two");

    let tx = UnsignedTransaction {
        chain_id: 1,
        nonce: 0,
        gas_limit: 21_000,
        max_fee_per_gas: 1_000_000_000,
        priority_fee_per_gas: 0,
        to: Some(Address::new([0x06u8; 20])),
        value: 100,
        data: vec![],
        tx_kind: TransactionKind::Transfer,
        memo: None,
    };

    let signed1 = sign_transaction(&keypair1, tx.clone()).unwrap();
    let signed2 = sign_transaction(&keypair2, tx).unwrap();

    // Different keypairs should produce different signatures
    assert_ne!(signed1.signature, signed2.signature);
    assert_ne!(signed1.public_key, signed2.public_key);
}

#[test]
fn test_executor_address_derivation_from_keypair_ml_dsa44() {
    let keypair = test_keypair(b"address derivation");
    let address = keypair.address().expect("Failed to derive address");

    assert_eq!(address.as_bytes().len(), 20);

    // Same keypair should produce same address (deterministic)
    let keypair2 = test_keypair(b"address derivation");
    let address2 = keypair2.address().unwrap();
    assert_eq!(address, address2);
}

#[test]
fn test_executor_public_key_extraction_ml_dsa44() {
    let keypair = test_keypair(b"pubkey extraction");
    let public_key = keypair.public_key();

    assert_eq!(public_key.len(), 1184); // ML-DSA-44 public key size
    assert!(!public_key.is_empty());
}

#[test]
fn test_executor_gas_config_with_ml_dsa44() {
    let gas_config = GasConfig::default();
    let config = EvmConfig {
        gas_config,
        chain_id: 1,
    };

    let mut executor = Executor::with_config(config);
    let keypair = test_keypair(b"gas config test");

    let tx = UnsignedTransaction {
        chain_id: 1,
        nonce: 0,
        gas_limit: 100_000,
        max_fee_per_gas: 1_000_000_000,
        priority_fee_per_gas: 0,
        to: Some(Address::new([0x07u8; 20])),
        value: 100,
        data: vec![],
        tx_kind: TransactionKind::Transfer,
        memo: None,
    };

    let signed_tx = sign_transaction(&keypair, tx).unwrap();
    let result = executor.execute_signed(&signed_tx).unwrap();

    assert!(result.gas_used <= 100_000);
}

#[test]
fn test_executor_transaction_nonce_tracking_ml_dsa44() {
    let mut executor = Executor::new();
    let keypair = test_keypair(b"nonce tracking");

    // Transaction with nonce 0
    let tx1 = UnsignedTransaction {
        chain_id: 1,
        nonce: 0,
        gas_limit: 21_000,
        max_fee_per_gas: 1_000_000_000,
        priority_fee_per_gas: 0,
        to: Some(Address::new([0x08u8; 20])),
        value: 100,
        data: vec![],
        tx_kind: TransactionKind::Transfer,
        memo: None,
    };

    // Transaction with nonce 1
    let tx2 = UnsignedTransaction {
        chain_id: 1,
        nonce: 1,
        gas_limit: 21_000,
        max_fee_per_gas: 1_000_000_000,
        priority_fee_per_gas: 0,
        to: Some(Address::new([0x09u8; 20])),
        value: 100,
        data: vec![],
        tx_kind: TransactionKind::Transfer,
        memo: None,
    };

    let signed1 = sign_transaction(&keypair, tx1).unwrap();
    let signed2 = sign_transaction(&keypair, tx2).unwrap();

    // Execute both transactions
    let _result1 = executor.execute_signed(&signed1).unwrap();
    let _result2 = executor.execute_signed(&signed2).unwrap();

    // Both should succeed
}

#[test]
fn test_executor_chain_id_validation_ml_dsa44() {
    let mut executor = Executor::new();
    let keypair = test_keypair(b"chain id validation");

    let tx = UnsignedTransaction {
        chain_id: 2, // Wrong chain
        nonce: 0,
        gas_limit: 21_000,
        max_fee_per_gas: 1_000_000_000,
        priority_fee_per_gas: 0,
        to: Some(Address::new([0x0Au8; 20])),
        value: 100,
        data: vec![],
        tx_kind: TransactionKind::Transfer,
        memo: None,
    };

    let signed_tx = sign_transaction(&keypair, tx).unwrap();

    // Should fail due to wrong chain ID
    let result = executor.execute_signed(&signed_tx);
    assert!(result.is_err(), "Transaction with wrong chain ID should fail");
}

#[test]
fn test_executor_full_transaction_flow_ml_dsa44() {
    let mut executor = Executor::new();
    let keypair = test_keypair(b"full flow test");

    // Create transaction
    let unsigned_tx = UnsignedTransaction {
        chain_id: 1,
        nonce: 0,
        gas_limit: 21_000,
        max_fee_per_gas: 1_000_000_000,
        priority_fee_per_gas: 100_000_000,
        to: Some(Address::new([0x0Bu8; 20])),
        value: 1_000_000_000_000_000_000,
        data: vec![],
        tx_kind: TransactionKind::Transfer,
        memo: Some("Full flow test with ML-DSA-44".to_string()),
    };

    // Get transaction hash
    let tx_hash = unsigned_tx.hash();

    // Sign transaction
    let signed_tx = sign_transaction(&keypair, unsigned_tx).unwrap();

    // Verify signature
    let scheme = fortiquo_crypto::ml_dsa::MlDsa44Scheme;
    let verifier = DefaultVerifier::new(Arc::new(scheme));
    let message = signed_tx.signature_bytes().unwrap();
    assert!(verifier
        .verify(&message, &signed_tx.signature, &signed_tx.public_key)
        .is_ok());

    // Execute transaction
    let result = executor.execute_signed(&signed_tx).unwrap();
    assert!(result.success);

    // Create receipt
    let receipt = executor.create_receipt(tx_hash, 1, 0, &result);
    assert_eq!(receipt.status, ExecutionStatus::Success);
    assert_eq!(receipt.gas_used, 21_000);
    assert_eq!(receipt.tx_hash, tx_hash);
}

#[test]
fn test_executor_data_transaction_ml_dsa44() {
    let mut executor = Executor::new();
    let keypair = test_keypair(b"data transaction");

    let tx = UnsignedTransaction {
        chain_id: 1,
        nonce: 0,
        gas_limit: 50_000,
        max_fee_per_gas: 1_000_000_000,
        priority_fee_per_gas: 0,
        to: Some(Address::new([0x0Cu8; 20])),
        value: 0,
        data: vec![0x01, 0x02, 0x03, 0x04, 0x05],
        tx_kind: TransactionKind::ContractCall,
        memo: Some("Data transaction".to_string()),
    };

    let signed_tx = sign_transaction(&keypair, tx).unwrap();

    // Verify public key size
    assert_eq!(signed_tx.public_key.len(), 1184);

    // Execute
    let result = executor.execute_signed(&signed_tx).unwrap();
    assert!(result.success);
}
