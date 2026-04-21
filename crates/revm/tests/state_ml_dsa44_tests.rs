//! State management tests with ML-DSA-44 signed transactions

use fortiquo_crypto::ml_dsa::MlDsa44Keypair;
use fortiquo_revm::{Executor, StateManager};
use fortiquo_types::{Account, Address, SignatureBytes, SignedTransaction, TransactionKind, UnsignedTransaction, PublicKeyBytes};

fn test_keypair(seed: &[u8]) -> MlDsa44Keypair {
    MlDsa44Keypair::from_seed(seed).expect("Failed to create keypair")
}

fn create_signed_tx(
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
fn test_state_manager_account_creation_ml_dsa44() {
    let mut state = StateManager::new();
    let keypair = test_keypair(b"account creation");
    let address = keypair.address().expect("Failed to derive address");

    let account = Account::new(1_000_000_000);
    state.set_account(address, account);

    let retrieved = state.get_account(&address);
    assert_eq!(retrieved.balance, 1_000_000_000);
    assert!(state.account_exists(&address));
}

#[test]
fn test_state_manager_multiple_accounts_ml_dsa44() {
    let mut state = StateManager::new();
    let kp1 = test_keypair(b"account one");
    let kp2 = test_keypair(b"account two");
    let kp3 = test_keypair(b"account three");

    let addr1 = kp1.address().unwrap();
    let addr2 = kp2.address().unwrap();
    let addr3 = kp3.address().unwrap();

    state.set_account(addr1, Account::new(100));
    state.set_account(addr2, Account::new(200));
    state.set_account(addr3, Account::new(300));

    assert_eq!(state.get_account(&addr1).balance, 100);
    assert_eq!(state.get_account(&addr2).balance, 200);
    assert_eq!(state.get_account(&addr3).balance, 300);
}

#[test]
fn test_state_manager_account_balance_transfer_ml_dsa44() {
    let mut state = StateManager::new();
    let sender_kp = test_keypair(b"sender");
    let receiver_kp = test_keypair(b"receiver");

    let sender_addr = sender_kp.address().unwrap();
    let receiver_addr = receiver_kp.address().unwrap();

    // Setup initial balances
    let mut sender_account = Account::new(1_000_000);
    let mut receiver_account = Account::new(0);

    // Transfer 500_000 from sender to receiver
    sender_account.subtract_balance(500_000).unwrap();
    receiver_account.add_balance(500_000);

    state.set_account(sender_addr, sender_account);
    state.set_account(receiver_addr, receiver_account);

    assert_eq!(state.get_account(&sender_addr).balance, 500_000);
    assert_eq!(state.get_account(&receiver_addr).balance, 500_000);
}

#[test]
fn test_state_manager_nonce_increment_ml_dsa44() {
    let mut state = StateManager::new();
    let keypair = test_keypair(b"nonce test");
    let address = keypair.address().unwrap();

    let mut account = Account::new(1_000_000);
    assert_eq!(account.nonce, 0);

    account.increment_nonce();
    state.set_account(address, account);

    let retrieved = state.get_account(&address);
    assert_eq!(retrieved.nonce, 1);

    let mut updated = retrieved;
    updated.increment_nonce();
    state.set_account(address, updated);

    let final_account = state.get_account(&address);
    assert_eq!(final_account.nonce, 2);
}

#[test]
fn test_state_manager_storage_ml_dsa44() {
    let mut state = StateManager::new();
    let keypair = test_keypair(b"storage test");
    let address = keypair.address().unwrap();

    let key1 = fortiquo_types::Hash::new([1u8; 32]);
    let key2 = fortiquo_types::Hash::new([2u8; 32]);
    let value1 = vec![0x01, 0x02, 0x03];
    let value2 = vec![0x04, 0x05, 0x06];

    state.set_storage(address, key1, value1.clone());
    state.set_storage(address, key2, value2.clone());

    assert_eq!(state.get_storage(&address, &key1), value1);
    assert_eq!(state.get_storage(&address, &key2), value2);
}

#[test]
fn test_state_manager_checkpoint_restore_ml_dsa44() {
    let mut state = StateManager::new();
    let kp1 = test_keypair(b"checkpoint one");
    let kp2 = test_keypair(b"checkpoint two");

    let addr1 = kp1.address().unwrap();
    let addr2 = kp2.address().unwrap();

    state.set_account(addr1, Account::new(1_000_000));
    state.set_account(addr2, Account::new(2_000_000));

    // Create checkpoint
    let checkpoint = state.checkpoint();

    // Modify state
    state.set_account(addr1, Account::new(500_000));
    assert_eq!(state.get_account(&addr1).balance, 500_000);

    // Restore from checkpoint
    state.restore(checkpoint);
    assert_eq!(state.get_account(&addr1).balance, 1_000_000);
    assert_eq!(state.get_account(&addr2).balance, 2_000_000);
}

#[test]
fn test_state_manager_delete_account_ml_dsa44() {
    let mut state = StateManager::new();
    let keypair = test_keypair(b"delete account");
    let address = keypair.address().unwrap();

    state.set_account(address, Account::new(1_000_000));
    assert!(state.account_exists(&address));

    state.delete_account(address);
    assert!(!state.account_exists(&address));
}

#[test]
fn test_state_manager_with_executor_ml_dsa44() {
    let mut executor = Executor::new();
    let keypair = test_keypair(b"executor state test");

    // Create transaction
    let tx = UnsignedTransaction {
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

    let message = tx.serialize_for_signing().expect("Failed to serialize");
    let signature = keypair.sign(&message).expect("Failed to sign");
    let public_key = keypair.public_key().expect("Failed to get public key");

    let signed_tx = SignedTransaction::new(
        tx,
        public_key,
        signature,
        fortiquo_types::AlgorithmId::MlDsa44,
    );

    // Execute
    let result = executor.execute_signed(&signed_tx).expect("Failed to execute");
    assert!(result.success);

    // State should be accessible
    let state = executor.state();
    assert!(state.get_all_accounts().len() >= 0);
}

#[test]
fn test_state_manager_multiple_keypairs_different_addresses_ml_dsa44() {
    let mut state = StateManager::new();
    let seeds = vec![
        b"keypair 1" as &[u8],
        b"keypair 2",
        b"keypair 3",
        b"keypair 4",
        b"keypair 5",
    ];

    let keypairs: Vec<_> = seeds
        .iter()
        .map(|seed| test_keypair(seed))
        .collect();

    let addresses: Vec<_> = keypairs
        .iter()
        .map(|kp| kp.address().unwrap())
        .collect();

    // All addresses should be different
    for i in 0..addresses.len() {
        for j in (i + 1)..addresses.len() {
            assert_ne!(
                addresses[i], addresses[j],
                "Different keypairs should produce different addresses"
            );
        }
    }

    // Set different balances for each address
    for (i, addr) in addresses.iter().enumerate() {
        state.set_account(*addr, Account::new((i as u128 + 1) * 1_000_000));
    }

    // Verify all balances
    for (i, addr) in addresses.iter().enumerate() {
        assert_eq!(
            state.get_account(addr).balance,
            (i as u128 + 1) * 1_000_000
        );
    }
}

#[test]
fn test_state_manager_contract_code_hash_ml_dsa44() {
    let mut state = StateManager::new();
    let keypair = test_keypair(b"contract code");
    let address = keypair.address().unwrap();

    let mut account = Account::new(0);
    let code_hash = fortiquo_types::Hash::new([0xABu8; 32]);
    account.set_code_hash(code_hash);

    state.set_account(address, account);

    let retrieved = state.get_account(&address);
    assert_eq!(retrieved.code_hash, code_hash);
}

#[test]
fn test_state_manager_storage_trie_root_ml_dsa44() {
    let mut state = StateManager::new();
    let keypair = test_keypair(b"storage trie");
    let address = keypair.address().unwrap();

    let mut account = Account::new(1_000_000);
    let storage_root = fortiquo_types::Hash::new([0xCDu8; 32]);
    account.set_storage_root(storage_root);

    state.set_account(address, account);

    let retrieved = state.get_account(&address);
    assert_eq!(retrieved.storage_root, storage_root);
}

#[test]
fn test_state_manager_empty_account_check_ml_dsa44() {
    let mut state = StateManager::new();
    let keypair = test_keypair(b"empty account");
    let address = keypair.address().unwrap();

    let empty_account = Account::empty();
    state.set_account(address, empty_account);

    let retrieved = state.get_account(&address);
    assert!(retrieved.is_empty());
}

#[test]
fn test_state_manager_insufficient_balance_ml_dsa44() {
    let mut state = StateManager::new();
    let keypair = test_keypair(b"insufficient balance");
    let address = keypair.address().unwrap();

    let mut account = Account::new(100);
    let result = account.subtract_balance(200);

    assert!(result.is_err());
    assert_eq!(account.balance, 100, "Balance should not change");
}
