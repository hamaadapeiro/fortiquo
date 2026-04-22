//! Smoke integration between ML-DSA key material and the [`Executor`].

use fortiquo_crypto::Signer;
use fortiquo_crypto::MlDsa44Keypair;
use fortiquo_revm::Executor;
use fortiquo_types::{
    AlgorithmId, PublicKeyBytes, SignedTransaction, TransactionKind, UnsignedTransaction,
};

#[test]
fn test_wallet_style_sign_then_execute_signed() {
    // arrange
    let seed = b"wallet revm integration seed thirty two bytes";
    let kp = MlDsa44Keypair::from_seed(seed).unwrap();
    let sender = kp.address().unwrap();
    let mut executor = Executor::new();
    executor
        .state_mut()
        .set_account(sender, fortiquo_types::Account::new(10u128.pow(18)));

    let unsigned = UnsignedTransaction {
        chain_id: 1,
        nonce: 0,
        gas_limit: 21_000,
        max_fee_per_gas: 1,
        priority_fee_per_gas: 0,
        to: Some(fortiquo_types::Address::new([0xabu8; 20])),
        value: 0,
        data: vec![],
        tx_kind: TransactionKind::Transfer,
        memo: None,
    };
    let msg = unsigned.serialize_for_signing().unwrap();
    let sig = kp.sign(&msg).unwrap();
    let tx = SignedTransaction::new(
        unsigned,
        PublicKeyBytes::new(kp.public_key().as_slice().to_vec()),
        sig,
        AlgorithmId::MlDsa44,
    );

    // act
    let out = executor.execute_signed(&tx).unwrap();

    // assert
    assert!(out.success);
}
