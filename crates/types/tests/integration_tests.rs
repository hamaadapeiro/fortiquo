//! Integration tests for blockchain types

use fortiquo_types::{
    Account, Address, Block, BlockHash, BlockHeader, BlockNumber, ExecutionStatus, Hash, LogEntry,
    Receipt, SignedTransaction, TransactionKind, TxHash, UnsignedTransaction, ValidatorId,
};

#[test]
fn test_transaction_serialization_roundtrip() {
    let tx = UnsignedTransaction {
        chain_id: 1,
        nonce: 42,
        gas_limit: 21_000,
        max_fee_per_gas: 1_000_000_000,
        priority_fee_per_gas: 100_000_000,
        to: Some(Address::new([0x01u8; 20])),
        value: 1_000_000_000_000_000_000,
        data: vec![1, 2, 3, 4, 5],
        tx_kind: TransactionKind::Transfer,
        memo: Some("test transaction".to_string()),
    };

    // Serialize and deserialize
    let bytes = tx.serialize_for_signing().unwrap();
    let deserialized = postcard::from_bytes::<UnsignedTransaction>(&bytes).unwrap();

    assert_eq!(tx.chain_id, deserialized.chain_id);
    assert_eq!(tx.nonce, deserialized.nonce);
    assert_eq!(tx.gas_limit, deserialized.gas_limit);
    assert_eq!(tx.value, deserialized.value);
}

#[test]
fn test_signed_transaction_full_flow() {
    let unsigned = UnsignedTransaction {
        chain_id: 1,
        nonce: 0,
        gas_limit: 21_000,
        max_fee_per_gas: 1_000_000_000,
        priority_fee_per_gas: 0,
        to: Some(Address::new([0x02u8; 20])),
        value: 100,
        data: vec![],
        tx_kind: TransactionKind::Transfer,
        memo: None,
    };

    let signed = SignedTransaction::new(
        unsigned.clone(),
        fortiquo_types::PublicKeyBytes::new(vec![42u8; 1184]),
        fortiquo_types::SignatureBytes::new(vec![99u8; 4668]),
        fortiquo_types::AlgorithmId::MlDsa44,
    );

    let serialized = signed.serialize().unwrap();
    let deserialized = SignedTransaction::deserialize(&serialized).unwrap();

    assert_eq!(signed.unsigned_tx.chain_id, deserialized.unsigned_tx.chain_id);
    assert_eq!(signed.public_key, deserialized.public_key);
    assert_eq!(signed.signature, deserialized.signature);
    assert_eq!(signed.algorithm_id, deserialized.algorithm_id);
}

#[test]
fn test_block_header_with_poh() {
    let header = BlockHeader {
        number: 1,
        parent_hash: BlockHash::new(Hash::new([1u8; 32])),
        state_root: Hash::new([2u8; 32]),
        tx_root: Hash::new([3u8; 32]),
        receipts_root: Hash::new([4u8; 32]),
        poh_start_hash: Hash::new([5u8; 32]),
        poh_end_hash: Hash::new([6u8; 32]),
        poh_start_tick: 0,
        poh_end_tick: 399,
        leader_id: ValidatorId::new([7u8; 32]),
        timestamp: 1_000_000,
        gas_used: 50_000,
        gas_limit: 100_000,
    };

    let hash1 = header.hash();
    let hash2 = header.hash();
    assert_eq!(hash1, hash2, "Header hash should be deterministic");
}

#[test]
fn test_block_with_transactions() {
    let header = BlockHeader {
        number: 1,
        parent_hash: BlockHash::new(Hash::new([1u8; 32])),
        state_root: Hash::new([2u8; 32]),
        tx_root: Hash::new([3u8; 32]),
        receipts_root: Hash::new([4u8; 32]),
        poh_start_hash: Hash::new([5u8; 32]),
        poh_end_hash: Hash::new([6u8; 32]),
        poh_start_tick: 0,
        poh_end_tick: 399,
        leader_id: ValidatorId::new([7u8; 32]),
        timestamp: 1_000_000,
        gas_used: 42_000,
        gas_limit: 100_000,
    };

    let tx = UnsignedTransaction {
        chain_id: 1,
        nonce: 0,
        gas_limit: 21_000,
        max_fee_per_gas: 1_000_000_000,
        priority_fee_per_gas: 0,
        to: Some(Address::new([0x03u8; 20])),
        value: 0,
        data: vec![],
        tx_kind: TransactionKind::Transfer,
        memo: None,
    };

    let signed_tx = SignedTransaction::new(
        tx,
        fortiquo_types::PublicKeyBytes::new(vec![42u8; 1184]),
        fortiquo_types::SignatureBytes::new(vec![99u8; 4668]),
        fortiquo_types::AlgorithmId::MlDsa44,
    );

    let body = fortiquo_types::BlockBody {
        poh_entries: vec![],
        signed_transactions: vec![signed_tx],
    };

    let block = Block::new(header, body);
    assert_eq!(block.tx_count(), 1);
    assert_eq!(block.poh_entry_count(), 0);
}

#[test]
fn test_receipt_with_logs() {
    let tx_hash = TxHash::new(Hash::new([1u8; 32]));
    let block_hash = BlockHash::new(Hash::new([2u8; 32]));
    let mut receipt = Receipt::new(tx_hash, 1, block_hash, 0, ExecutionStatus::Success, 50_000);

    // Add logs
    let addr = Address::new([5u8; 20]);
    let log = LogEntry::new(
        addr,
        vec![Hash::new([8u8; 32]), Hash::new([9u8; 32])],
        vec![1, 2, 3, 4, 5],
    );
    receipt.add_log(log);

    assert_eq!(receipt.logs.len(), 1);
    assert_eq!(receipt.logs[0].topics.len(), 2);
    assert!(receipt.is_success());
}

#[test]
fn test_account_lifecycle() {
    let mut account = Account::new(1_000_000_000_000_000_000); // 1 ether

    assert_eq!(account.nonce, 0);
    assert_eq!(account.balance, 1_000_000_000_000_000_000);
    assert!(account.is_empty() == false);

    // Increment nonce
    account.increment_nonce();
    assert_eq!(account.nonce, 1);

    // Subtract balance
    account.subtract_balance(100_000_000_000_000_000).unwrap();
    assert_eq!(account.balance, 900_000_000_000_000_000);

    // Add balance
    account.add_balance(50_000_000_000_000_000);
    assert_eq!(account.balance, 950_000_000_000_000_000);
}

#[test]
fn test_account_insufficient_balance() {
    let mut account = Account::new(100);
    let result = account.subtract_balance(200);
    assert!(result.is_err());
    assert_eq!(account.balance, 100, "Balance should not change on error");
}

#[test]
fn test_different_transaction_kinds() {
    let transfer = UnsignedTransaction {
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

    let contract_call = UnsignedTransaction {
        chain_id: 1,
        nonce: 1,
        gas_limit: 100_000,
        max_fee_per_gas: 1_000_000_000,
        priority_fee_per_gas: 0,
        to: Some(Address::new([0x02u8; 20])),
        value: 0,
        data: vec![0x01, 0x02, 0x03],
        tx_kind: TransactionKind::ContractCall,
        memo: None,
    };

    let contract_create = UnsignedTransaction {
        chain_id: 1,
        nonce: 2,
        gas_limit: 200_000,
        max_fee_per_gas: 1_000_000_000,
        priority_fee_per_gas: 0,
        to: None,
        value: 0,
        data: vec![0x60, 0x80, 0x60, 0x40], // EVM bytecode example
        tx_kind: TransactionKind::ContractCreate,
        memo: None,
    };

    assert_eq!(transfer.tx_kind, TransactionKind::Transfer);
    assert_eq!(contract_call.tx_kind, TransactionKind::ContractCall);
    assert_eq!(contract_create.tx_kind, TransactionKind::ContractCreate);
}

#[test]
fn test_hash_display_format() {
    let hash = Hash::new([0x12u8; 32]);
    let formatted = format!("{}", hash);
    assert!(formatted.starts_with("0x"));
    assert_eq!(formatted.len(), 66); // 0x + 64 hex chars
}

#[test]
fn test_address_display_format() {
    let addr = Address::new([0xAB as u8; 20]);
    let formatted = format!("{}", addr);
    assert!(formatted.starts_with("0x"));
    assert_eq!(formatted.len(), 42); // 0x + 40 hex chars
}

#[test]
fn test_block_serialization_roundtrip() {
    let header = BlockHeader {
        number: 5,
        parent_hash: BlockHash::new(Hash::new([1u8; 32])),
        state_root: Hash::new([2u8; 32]),
        tx_root: Hash::new([3u8; 32]),
        receipts_root: Hash::new([4u8; 32]),
        poh_start_hash: Hash::new([5u8; 32]),
        poh_end_hash: Hash::new([6u8; 32]),
        poh_start_tick: 0,
        poh_end_tick: 399,
        leader_id: ValidatorId::new([7u8; 32]),
        timestamp: 2_000_000,
        gas_used: 100_000,
        gas_limit: 200_000,
    };

    let body = fortiquo_types::BlockBody {
        poh_entries: vec![],
        signed_transactions: vec![],
    };

    let block = Block::new(header, body);
    let serialized = block.serialize().unwrap();
    let deserialized = Block::deserialize(&serialized).unwrap();

    assert_eq!(block.header.number, deserialized.header.number);
    assert_eq!(block.header.timestamp, deserialized.header.timestamp);
    assert_eq!(block.header.gas_used, deserialized.header.gas_used);
}
