//! Advanced execution tests with ML-DSA-44: receipts, logs, and complex scenarios

use fortiquo_crypto::ml_dsa::MlDsa44Keypair;
use fortiquo_revm::Executor;
use fortiquo_types::{
    Address, ExecutionStatus, Hash, LogEntry, Receipt, SignedTransaction, TransactionKind,
    UnsignedTransaction,
};

fn test_keypair(seed: &[u8]) -> MlDsa44Keypair {
    MlDsa44Keypair::from_seed(seed).expect("Failed to create keypair")
}

fn create_signed_transfer(
    keypair: &MlDsa44Keypair,
    nonce: u64,
    to: Address,
    value: u128,
) -> SignedTransaction {
    let unsigned = UnsignedTransaction {
        chain_id: 1,
        nonce,
        gas_limit: 21_000,
        max_fee_per_gas: 1_000_000_000,
        priority_fee_per_gas: 0,
        to: Some(to),
        value,
        data: vec![],
        tx_kind: TransactionKind::Transfer,
        memo: None,
    };

    let message = unsigned.serialize_for_signing().expect("Failed to serialize");
    let signature = keypair.sign(&message).expect("Failed to sign");
    let public_key = keypair.public_key().expect("Failed to get public key");

    SignedTransaction::new(
        unsigned,
        public_key,
        signature,
        fortiquo_types::AlgorithmId::MlDsa44,
    )
}

#[test]
fn test_receipt_with_logs_ml_dsa44() {
    let mut executor = Executor::new();
    let keypair = test_keypair(b"receipt with logs");
    let contract_addr = Address::new([0x01u8; 20]);

    let tx = create_signed_transfer(&keypair, 0, contract_addr, 0);
    let tx_hash = tx.hash();

    let result = executor.execute_signed(&tx).unwrap();

    let mut receipt = executor.create_receipt(tx_hash, 1, 0, &result);

    // Add logs
    let log1 = LogEntry::new(
        contract_addr,
        vec![Hash::new([0x01u8; 32]), Hash::new([0x02u8; 32])],
        vec![1, 2, 3, 4, 5],
    );

    let log2 = LogEntry::new(
        contract_addr,
        vec![Hash::new([0x03u8; 32])],
        vec![6, 7, 8],
    );

    receipt.add_log(log1);
    receipt.add_log(log2);

    assert_eq!(receipt.logs.len(), 2);
    assert_eq!(receipt.logs[0].topics.len(), 2);
    assert_eq!(receipt.logs[1].topics.len(), 1);
    assert_eq!(receipt.logs[0].data, vec![1, 2, 3, 4, 5]);
}

#[test]
fn test_receipt_contract_creation_ml_dsa44() {
    let mut executor = Executor::new();
    let keypair = test_keypair(b"contract creation receipt");
    let created_addr = Address::new([0x02u8; 20]);

    let unsigned = UnsignedTransaction {
        chain_id: 1,
        nonce: 0,
        gas_limit: 32_000,
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
    let public_key = keypair.public_key().unwrap();

    let signed_tx = SignedTransaction::new(
        unsigned,
        public_key,
        signature,
        fortiquo_types::AlgorithmId::MlDsa44,
    );

    let tx_hash = signed_tx.hash();
    let result = executor.execute_signed(&signed_tx).unwrap();

    let mut receipt = executor.create_receipt(tx_hash, 1, 0, &result);
    receipt.set_contract_address(created_addr);

    assert_eq!(receipt.contract_address, Some(created_addr));
    assert_eq!(receipt.status, ExecutionStatus::Success);
}

#[test]
fn test_multiple_receipts_ml_dsa44() {
    let mut executor = Executor::new();
    let kp1 = test_keypair(b"tx1 receipt");
    let kp2 = test_keypair(b"tx2 receipt");
    let kp3 = test_keypair(b"tx3 receipt");

    let tx1 = create_signed_transfer(&kp1, 0, Address::new([0x03u8; 20]), 100);
    let tx2 = create_signed_transfer(&kp2, 0, Address::new([0x04u8; 20]), 200);
    let tx3 = create_signed_transfer(&kp3, 0, Address::new([0x05u8; 20]), 300);

    let hash1 = tx1.hash();
    let hash2 = tx2.hash();
    let hash3 = tx3.hash();

    let result1 = executor.execute_signed(&tx1).unwrap();
    let result2 = executor.execute_signed(&tx2).unwrap();
    let result3 = executor.execute_signed(&tx3).unwrap();

    let receipt1 = executor.create_receipt(hash1, 1, 0, &result1);
    let receipt2 = executor.create_receipt(hash2, 1, 1, &result2);
    let receipt3 = executor.create_receipt(hash3, 1, 2, &result3);

    assert_eq!(receipt1.tx_index, 0);
    assert_eq!(receipt2.tx_index, 1);
    assert_eq!(receipt3.tx_index, 2);
    assert_eq!(receipt1.gas_used, 21_000);
    assert_eq!(receipt2.gas_used, 21_000);
    assert_eq!(receipt3.gas_used, 21_000);
}

#[test]
fn test_receipt_cumulative_gas_ml_dsa44() {
    let mut executor = Executor::new();
    let keypair = test_keypair(b"cumulative gas");

    let tx = create_signed_transfer(&keypair, 0, Address::new([0x06u8; 20]), 50);
    let tx_hash = tx.hash();

    let result = executor.execute_signed(&tx).unwrap();
    let mut receipt = executor.create_receipt(tx_hash, 1, 0, &result);

    receipt.set_cumulative_gas(21_000);
    assert_eq!(receipt.cumulative_gas_used, 21_000);

    receipt.set_cumulative_gas(42_000);
    assert_eq!(receipt.cumulative_gas_used, 42_000);
}

#[test]
fn test_receipt_output_data_ml_dsa44() {
    let mut executor = Executor::new();
    let keypair = test_keypair(b"output data");

    let tx = create_signed_transfer(&keypair, 0, Address::new([0x07u8; 20]), 0);
    let tx_hash = tx.hash();

    let result = executor.execute_signed(&tx).unwrap();
    let mut receipt = executor.create_receipt(tx_hash, 1, 0, &result);

    let output = vec![0x01, 0x02, 0x03, 0x04, 0x05];
    receipt.set_output(output.clone());

    assert_eq!(receipt.output, output);
}

#[test]
fn test_complex_log_entry_ml_dsa44() {
    let contract_addr = Address::new([0x08u8; 20]);
    let topics = vec![
        Hash::new([0x01u8; 32]),
        Hash::new([0x02u8; 32]),
        Hash::new([0x03u8; 32]),
    ];
    let data = vec![
        0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D,
        0x0E, 0x0F,
    ];

    let log = LogEntry::new(contract_addr, topics.clone(), data.clone());

    assert_eq!(log.address, contract_addr);
    assert_eq!(log.topics.len(), 3);
    assert_eq!(log.data, data);
}

#[test]
fn test_block_1_transactions_ml_dsa44() {
    let mut executor = Executor::new();
    let keypairs: Vec<_> = (0..5)
        .map(|i| test_keypair(format!("keypair {}", i).as_bytes()))
        .collect();

    let mut receipts = Vec::new();

    for (idx, keypair) in keypairs.iter().enumerate() {
        let tx = create_signed_transfer(
            keypair,
            0,
            Address::new([idx as u8; 20]),
            100 * (idx as u128 + 1),
        );

        let tx_hash = tx.hash();
        let result = executor.execute_signed(&tx).unwrap();
        let receipt = executor.create_receipt(tx_hash, 1, idx as u32, &result);

        receipts.push(receipt);
    }

    assert_eq!(receipts.len(), 5);
    for (i, receipt) in receipts.iter().enumerate() {
        assert_eq!(receipt.tx_index, i as u32);
        assert_eq!(receipt.block_number, 1);
    }
}

#[test]
fn test_receipt_serialization_ml_dsa44() {
    let mut executor = Executor::new();
    let keypair = test_keypair(b"receipt serialization");

    let tx = create_signed_transfer(&keypair, 0, Address::new([0x09u8; 20]), 100);
    let tx_hash = tx.hash();

    let result = executor.execute_signed(&tx).unwrap();
    let receipt = executor.create_receipt(tx_hash, 1, 0, &result);

    let serialized = receipt.serialize().expect("Failed to serialize receipt");
    let deserialized = Receipt::deserialize(&serialized).expect("Failed to deserialize receipt");

    assert_eq!(receipt.tx_hash, deserialized.tx_hash);
    assert_eq!(receipt.block_number, deserialized.block_number);
    assert_eq!(receipt.gas_used, deserialized.gas_used);
    assert_eq!(receipt.status, deserialized.status);
}

#[test]
fn test_transaction_sequence_with_receipts_ml_dsa44() {
    let mut executor = Executor::new();
    let sender = test_keypair(b"sender sequence");

    let mut receipts = Vec::new();

    // Execute multiple transactions from same sender
    for nonce in 0..3 {
        let tx = UnsignedTransaction {
            chain_id: 1,
            nonce,
            gas_limit: 21_000,
            max_fee_per_gas: 1_000_000_000,
            priority_fee_per_gas: 0,
            to: Some(Address::new([nonce as u8; 20])),
            value: 100 * (nonce as u128 + 1),
            data: vec![],
            tx_kind: TransactionKind::Transfer,
            memo: None,
        };

        let message = tx.serialize_for_signing().unwrap();
        let signature = sender.sign(&message).unwrap();
        let public_key = sender.public_key().unwrap();

        let signed_tx = SignedTransaction::new(
            tx,
            public_key,
            signature,
            fortiquo_types::AlgorithmId::MlDsa44,
        );

        let tx_hash = signed_tx.hash();
        let result = executor.execute_signed(&signed_tx).unwrap();
        let receipt = executor.create_receipt(tx_hash, 1, nonce as u32, &result);

        receipts.push(receipt);
    }

    assert_eq!(receipts.len(), 3);
    for (i, receipt) in receipts.iter().enumerate() {
        assert_eq!(receipt.tx_index, i as u32);
        assert!(receipt.is_success());
    }
}

#[test]
fn test_ml_dsa44_signature_in_receipt_ml_dsa44() {
    let mut executor = Executor::new();
    let keypair = test_keypair(b"signature in receipt");

    let tx = create_signed_transfer(&keypair, 0, Address::new([0x0Au8; 20]), 50);
    let public_key = keypair.public_key().unwrap();

    // Verify the public key size
    assert_eq!(public_key.len(), 1184);

    let tx_hash = tx.hash();
    let result = executor.execute_signed(&tx).unwrap();
    let receipt = executor.create_receipt(tx_hash, 1, 0, &result);

    // Receipt should have proper structure
    assert_eq!(receipt.tx_hash, fortiquo_types::TxHash::new(tx_hash));
    assert_eq!(receipt.status, ExecutionStatus::Success);
}

#[test]
fn test_execution_with_zero_value_ml_dsa44() {
    let mut executor = Executor::new();
    let keypair = test_keypair(b"zero value execution");

    let tx = create_signed_transfer(&keypair, 0, Address::new([0x0Bu8; 20]), 0);
    let tx_hash = tx.hash();

    let result = executor.execute_signed(&tx).unwrap();
    assert!(result.success);

    let receipt = executor.create_receipt(tx_hash, 1, 0, &result);
    assert_eq!(receipt.status, ExecutionStatus::Success);
}

#[test]
fn test_multi_topic_logs_ml_dsa44() {
    let contract_addr = Address::new([0x0Cu8; 20]);

    // Log with 0 topics
    let log0 = LogEntry::new(contract_addr, vec![], vec![1, 2, 3]);
    assert_eq!(log0.topics.len(), 0);

    // Log with 1 topic
    let log1 = LogEntry::new(
        contract_addr,
        vec![Hash::new([0x01u8; 32])],
        vec![1, 2, 3],
    );
    assert_eq!(log1.topics.len(), 1);

    // Log with 3 topics
    let log3 = LogEntry::new(
        contract_addr,
        vec![
            Hash::new([0x01u8; 32]),
            Hash::new([0x02u8; 32]),
            Hash::new([0x03u8; 32]),
        ],
        vec![1, 2, 3],
    );
    assert_eq!(log3.topics.len(), 3);

    // Log with 4 topics
    let log4 = LogEntry::new(
        contract_addr,
        vec![
            Hash::new([0x01u8; 32]),
            Hash::new([0x02u8; 32]),
            Hash::new([0x03u8; 32]),
            Hash::new([0x04u8; 32]),
        ],
        vec![1, 2, 3],
    );
    assert_eq!(log4.topics.len(), 4);
}
