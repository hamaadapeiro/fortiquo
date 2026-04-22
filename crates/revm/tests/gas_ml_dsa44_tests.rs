//! Gas accounting and execution tests with ML-DSA-44

use fortiquo_crypto::ml_dsa::MlDsa44Keypair;
use fortiquo_crypto::Signer;
use fortiquo_revm::{Executor, EvmConfig, GasConfig, ExecutionResult};
use fortiquo_types::{
    Address, ExecutionStatus, SignedTransaction, TransactionKind, UnsignedTransaction,
};

fn test_keypair(seed: &[u8]) -> MlDsa44Keypair {
    MlDsa44Keypair::from_seed(seed).expect("Failed to create keypair")
}

fn sign_tx(
    keypair: &MlDsa44Keypair,
    nonce: u64,
    gas_limit: u64,
    max_fee: u128,
    to: Address,
    value: u128,
) -> SignedTransaction {
    let unsigned = UnsignedTransaction {
        chain_id: 1,
        nonce,
        gas_limit,
        max_fee_per_gas: max_fee,
        priority_fee_per_gas: 0,
        to: Some(to),
        value,
        data: vec![],
        tx_kind: TransactionKind::Transfer,
        memo: None,
    };

    let message = unsigned.serialize_for_signing().expect("Failed to serialize");
    let signature = keypair.sign(&message).expect("Failed to sign");
    let public_key = keypair.public_key().clone();

    SignedTransaction::new(
        unsigned,
        public_key,
        signature,
        fortiquo_types::AlgorithmId::MlDsa44,
    )
}

#[test]
fn test_gas_config_default_ml_dsa44() {
    let config = GasConfig::default();

    assert_eq!(config.gas_add, 3);
    assert_eq!(config.gas_mul, 5);
    assert_eq!(config.gas_sub, 3);
    assert_eq!(config.gas_sstore, 20_000);
    assert_eq!(config.gas_sload, 200);
}

#[test]
fn test_gas_validation_ml_dsa44() {
    let config = GasConfig::default();

    // Valid: used < limit
    assert!(config.validate_gas(50_000, 100_000).is_ok());

    // Valid: used == limit
    assert!(config.validate_gas(100_000, 100_000).is_ok());

    // Invalid: used > limit
    assert!(config.validate_gas(150_000, 100_000).is_err());
}

#[test]
fn test_memory_expansion_gas_ml_dsa44() {
    // No expansion
    assert_eq!(GasConfig::gas_memory_expansion(100, 100), 0);

    // Contraction (should be free)
    assert_eq!(GasConfig::gas_memory_expansion(50, 100), 0);

    // Expansion
    let cost = GasConfig::gas_memory_expansion(200, 100);
    assert!(cost > 0);

    // Larger expansion costs more
    let larger_cost = GasConfig::gas_memory_expansion(500, 100);
    assert!(larger_cost > cost);
}

#[test]
fn test_executor_transfer_gas_accounting_ml_dsa44() {
    let mut executor = Executor::new();
    let keypair = test_keypair(b"gas accounting");

    let tx = sign_tx(
        &keypair,
        0,
        21_000,
        1_000_000_000,
        Address::new([0x01u8; 20]),
        100,
    );

    let result = executor.execute_signed(&tx).unwrap();
    assert!(result.success);
    assert_eq!(result.gas_used, 21_000); // Base transfer cost
}

#[test]
fn test_executor_contract_creation_gas_ml_dsa44() {
    let mut executor = Executor::new();
    let keypair = test_keypair(b"contract creation gas");

    let unsigned = UnsignedTransaction {
        chain_id: 1,
        nonce: 0,
        gas_limit: 50_000,
        max_fee_per_gas: 1_000_000_000,
        priority_fee_per_gas: 0,
        to: None, // Contract creation
        value: 0,
        data: vec![0x60, 0x80, 0x60, 0x40],
        tx_kind: TransactionKind::ContractCreate,
        memo: None,
    };

    let message = unsigned.serialize_for_signing().unwrap();
    let signature = keypair.sign(&message).unwrap();
    let public_key = keypair.public_key().clone();

    let signed_tx = SignedTransaction::new(
        unsigned,
        public_key,
        signature,
        fortiquo_types::AlgorithmId::MlDsa44,
    );

    let result = executor.execute_signed(&signed_tx).unwrap();
    assert!(result.success);
}

#[test]
fn test_executor_custom_gas_config_ml_dsa44() {
    let mut custom_gas = GasConfig::default();
    custom_gas.gas_add = 10; // Higher cost
    custom_gas.gas_sstore = 50_000; // Higher storage cost

    let config = EvmConfig {
        gas_config: custom_gas,
        chain_id: 1,
    };

    let mut executor = Executor::with_config(config);
    let keypair = test_keypair(b"custom gas config");

    let tx = sign_tx(
        &keypair,
        0,
        100_000,
        1_000_000_000,
        Address::new([0x02u8; 20]),
        50,
    );

    let result = executor.execute_signed(&tx).unwrap();
    assert!(result.success);
}

#[test]
fn test_executor_gas_limit_enforcement_ml_dsa44() {
    let mut executor = Executor::new();
    let keypair = test_keypair(b"gas limit enforcement");

    // Very low gas limit
    let tx = sign_tx(
        &keypair,
        0,
        1, // Too low
        1_000_000_000,
        Address::new([0x03u8; 20]),
        100,
    );

    // Should fail validation
    let result = executor.execute_unsigned(&tx.unsigned_tx);
    assert!(result.is_err() || !result.unwrap().success);
}

#[test]
fn test_executor_receipt_gas_ml_dsa44() {
    let mut executor = Executor::new();
    let keypair = test_keypair(b"receipt gas");

    let tx = sign_tx(
        &keypair,
        0,
        30_000,
        1_000_000_000,
        Address::new([0x04u8; 20]),
        200,
    );

    let tx_hash = tx.hash();
    let result = executor.execute_signed(&tx).unwrap();
    let receipt = executor.create_receipt(tx_hash, 1, 0, &result);

    assert_eq!(receipt.gas_used, result.gas_used);
    assert_eq!(receipt.status, ExecutionStatus::Success);
}

#[test]
fn test_executor_multiple_transactions_gas_ml_dsa44() {
    let mut executor = Executor::new();
    let kp1 = test_keypair(b"tx1");
    let kp2 = test_keypair(b"tx2");
    let kp3 = test_keypair(b"tx3");

    let tx1 = sign_tx(&kp1, 0, 21_000, 1_000_000_000, Address::new([0x05u8; 20]), 100);
    let tx2 = sign_tx(&kp2, 0, 21_000, 1_000_000_000, Address::new([0x06u8; 20]), 200);
    let tx3 = sign_tx(&kp3, 0, 21_000, 1_000_000_000, Address::new([0x07u8; 20]), 300);

    let result1 = executor.execute_signed(&tx1).unwrap();
    let result2 = executor.execute_signed(&tx2).unwrap();
    let result3 = executor.execute_signed(&tx3).unwrap();

    assert!(result1.success);
    assert!(result2.success);
    assert!(result3.success);

    // All should have base transfer cost
    assert_eq!(result1.gas_used, 21_000);
    assert_eq!(result2.gas_used, 21_000);
    assert_eq!(result3.gas_used, 21_000);
}

#[test]
fn test_executor_zero_value_transfer_gas_ml_dsa44() {
    let mut executor = Executor::new();
    let keypair = test_keypair(b"zero value transfer");

    let tx = sign_tx(&keypair, 0, 21_000, 1_000_000_000, Address::new([0x08u8; 20]), 0);

    let result = executor.execute_signed(&tx).unwrap();
    assert!(result.success);
    assert_eq!(result.gas_used, 21_000);
}

#[test]
fn test_executor_with_data_gas_ml_dsa44() {
    let mut executor = Executor::new();
    let keypair = test_keypair(b"data gas");

    let unsigned = UnsignedTransaction {
        chain_id: 1,
        nonce: 0,
        gas_limit: 50_000,
        max_fee_per_gas: 1_000_000_000,
        priority_fee_per_gas: 0,
        to: Some(Address::new([0x09u8; 20])),
        value: 0,
        data: vec![0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08],
        tx_kind: TransactionKind::ContractCall,
        memo: None,
    };

    let message = unsigned.serialize_for_signing().unwrap();
    let signature = keypair.sign(&message).unwrap();
    let public_key = keypair.public_key().clone();

    let signed_tx = SignedTransaction::new(
        unsigned,
        public_key,
        signature,
        fortiquo_types::AlgorithmId::MlDsa44,
    );

    let result = executor.execute_signed(&signed_tx).unwrap();
    assert!(result.success);
}

#[test]
fn test_gas_config_opcode_costs_ml_dsa44() {
    let config = GasConfig::default();

    // Verify specific opcode costs
    assert_eq!(config.gas_stop, 0);
    assert_eq!(config.gas_add, 3);
    assert_eq!(config.gas_mul, 5);
    assert_eq!(config.gas_div, 5);
    assert_eq!(config.gas_sstore, 20_000);
    assert_eq!(config.gas_sload, 200);
    assert_eq!(config.gas_call, 700);
    assert_eq!(config.gas_create, 32_000);
}

#[test]
fn test_execution_result_success_ml_dsa44() {
    let result = ExecutionResult::success(21_000, vec![1, 2, 3]);

    assert!(result.success);
    assert_eq!(result.gas_used, 21_000);
    assert_eq!(result.output, vec![1, 2, 3]);
    assert!(result.logs.is_empty());
}

#[test]
fn test_execution_result_reverted_ml_dsa44() {
    let reason = b"execution reverted".to_vec();
    let result = ExecutionResult::reverted(50_000, reason.clone());

    assert!(!result.success);
    assert_eq!(result.gas_used, 50_000);
    assert_eq!(result.output, reason);
}

#[test]
fn test_executor_chain_config_ml_dsa44() {
    let config = EvmConfig {
        gas_config: GasConfig::default(),
        chain_id: 137, // Polygon
    };

    let executor = Executor::with_config(config);
    assert_eq!(executor.config().chain_id, 137);
}

#[test]
fn test_executor_default_config_ml_dsa44() {
    let executor = Executor::new();
    assert_eq!(executor.config().chain_id, 1);
}

#[test]
fn test_gas_expansion_calculation_ml_dsa44() {
    // Test memory expansion gas calculation
    let test_cases = vec![
        (0, 0, 0),       // No change
        (32, 0, 3),      // Expand by 32 bytes
        (64, 0, 12),     // Expand by 64 bytes
        (256, 0, 732),   // Expand by 256 bytes
    ];

    for (new_size, old_size, expected_min) in test_cases {
        let cost = GasConfig::gas_memory_expansion(new_size, old_size);
        assert!(cost >= expected_min, "Memory expansion cost too low");
    }
}

#[test]
fn test_high_gas_limit_ml_dsa44() {
    let mut executor = Executor::new();
    let keypair = test_keypair(b"high gas limit");

    let tx = sign_tx(
        &keypair,
        0,
        30_000_000, // Max block gas
        1_000_000_000,
        Address::new([0x0Au8; 20]),
        100,
    );

    let result = executor.execute_signed(&tx).unwrap();
    assert!(result.success);
}
