//! Transaction pool with ML-DSA-44 admission checks and fee-ordered selection.
//!
//! The pool indexes pending work by `(sender, nonce)` and prioritizes inclusion using
//! `max_fee_per_gas` tiers in a [`std::collections::BTreeMap`].

mod error;

pub use error::MempoolError;

use fortiquo_revm::Executor;
use fortiquo_state::StateStore;
use fortiquo_types::{Address, SignedTransaction, TxHash};
use std::collections::{BTreeMap, HashMap};

/// Maximum serialized signed transaction size accepted by [`Mempool::check_size`].
pub const MAX_SIGNED_TX_BYTES: usize = 512 * 1024;

/// In-memory transaction pool used before block production.
///
/// Admission enforces signature-derived sender, nonce, balance, and chain id against a
/// [`fortiquo_state::StateStore`]. Selection walks fee tiers from high to low.
pub struct Mempool {
    chain_id: u64,
    pending: HashMap<(Address, u64), SignedTransaction>,
    /// Ascending fee tier → transaction hashes (iterate with [`std::iter::DoubleEndedIterator::rev`] for block building).
    by_fee: BTreeMap<u128, Vec<TxHash>>,
    txs: HashMap<TxHash, SignedTransaction>,
}

impl Mempool {
    /// Create an empty pool for the given chain id.
    pub fn new(chain_id: u64) -> Self {
        Mempool {
            chain_id,
            pending: HashMap::new(),
            by_fee: BTreeMap::new(),
            txs: HashMap::new(),
        }
    }

    /// Full admission pipeline returning the canonical transaction hash.
    pub fn admit(
        &mut self,
        tx: SignedTransaction,
        state: &dyn StateStore,
    ) -> Result<TxHash, MempoolError> {
        let sender = self.verify_signature(&tx)?;
        if tx.unsigned_tx.chain_id != self.chain_id {
            return Err(MempoolError::InvalidChainId);
        }
        self.check_size(&tx)?;
        let h = tx.hash();
        self.check_duplicate(&h)?;
        if self
            .pending
            .contains_key(&(sender, tx.unsigned_tx.nonce))
        {
            return Err(MempoolError::NonceConflict);
        }
        self.check_nonce(sender, tx.unsigned_tx.nonce, state)?;
        self.check_balance(sender, &tx, state)?;

        let fee = tx.unsigned_tx.max_fee_per_gas;
        self.by_fee.entry(fee).or_default().push(h);
        self.pending
            .insert((sender, tx.unsigned_tx.nonce), tx.clone());
        self.txs.insert(h, tx);
        Ok(h)
    }

    /// Verify ML-DSA-44 and derive the sender [`Address`].
    pub fn verify_signature(&self, tx: &SignedTransaction) -> Result<Address, MempoolError> {
        Executor::verify_and_derive_sender(tx).map_err(|_| MempoolError::InvalidSignature)
    }

    /// Require `nonce >= on-chain account nonce` (stale lower nonces are not re-admitted).
    pub fn check_nonce(
        &self,
        sender: Address,
        nonce: u64,
        state: &dyn StateStore,
    ) -> Result<(), MempoolError> {
        let acc = state
            .get_account(&sender)
            .map_err(|e| MempoolError::State(e.to_string()))?;
        if nonce < acc.nonce {
            return Err(MempoolError::InvalidNonce);
        }
        Ok(())
    }

    /// Ensure balance covers `value + gas_limit * max_fee_per_gas`.
    pub fn check_balance(
        &self,
        sender: Address,
        tx: &SignedTransaction,
        state: &dyn StateStore,
    ) -> Result<(), MempoolError> {
        let acc = state
            .get_account(&sender)
            .map_err(|e| MempoolError::State(e.to_string()))?;
        let max_spend = tx
            .unsigned_tx
            .value
            .saturating_add(tx.unsigned_tx.gas_limit as u128 * tx.unsigned_tx.max_fee_per_gas);
        if acc.balance < max_spend {
            return Err(MempoolError::InsufficientBalance);
        }
        Ok(())
    }

    /// Bound postcard-encoded signed transaction size.
    pub fn check_size(&self, tx: &SignedTransaction) -> Result<(), MempoolError> {
        let n = postcard::to_allocvec(tx)
            .map_err(|e| MempoolError::State(e.to_string()))?
            .len();
        if n > MAX_SIGNED_TX_BYTES {
            return Err(MempoolError::TransactionTooLarge);
        }
        Ok(())
    }

    /// Reject if the hash is already tracked in the pool.
    pub fn check_duplicate(&self, tx_hash: &TxHash) -> Result<(), MempoolError> {
        if self.txs.contains_key(tx_hash) {
            return Err(MempoolError::DuplicateTransaction);
        }
        Ok(())
    }

    /// Greedy selection by descending `max_fee_per_gas` until `gas_budget` / `max_count`.
    pub fn select_transactions(&self, gas_budget: u64, max_count: usize) -> Vec<SignedTransaction> {
        let mut out = Vec::new();
        let mut gas_used = 0u64;
        for (_fee, hashes) in self.by_fee.iter().rev() {
            for h in hashes {
                if out.len() >= max_count {
                    return out;
                }
                let Some(tx) = self.txs.get(h) else {
                    continue;
                };
                let need = tx.unsigned_tx.gas_limit;
                if gas_used.saturating_add(need) > gas_budget {
                    continue;
                }
                gas_used = gas_used.saturating_add(need);
                out.push(tx.clone());
            }
        }
        out
    }

    /// Remove a transaction by hash if present.
    pub fn remove(&mut self, tx_hash: &TxHash) {
        let Some(tx) = self.txs.remove(tx_hash) else {
            return;
        };
        let sender = Executor::verify_and_derive_sender(&tx).ok();
        if let Some(sender) = sender {
            self.pending.remove(&(sender, tx.unsigned_tx.nonce));
        }
        let fee = tx.unsigned_tx.max_fee_per_gas;
        if let Some(vec) = self.by_fee.get_mut(&fee) {
            vec.retain(|x| x != tx_hash);
            if vec.is_empty() {
                self.by_fee.remove(&fee);
            }
        }
    }

    /// Number of transactions currently held.
    pub fn pending_count(&self) -> usize {
        self.txs.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fortiquo_crypto::Signer;
    use fortiquo_crypto::MlDsa44Keypair;
    use fortiquo_state::InMemoryStateStore;
    use fortiquo_types::{
        AlgorithmId, PublicKeyBytes, SignatureBytes, TransactionKind, UnsignedTransaction,
    };

    fn sample_tx(
        kp: &MlDsa44Keypair,
        nonce: u64,
        chain_id: u64,
        gas_limit: u64,
        max_fee: u128,
        value: u128,
    ) -> SignedTransaction {
        let unsigned = UnsignedTransaction {
            chain_id,
            nonce,
            gas_limit,
            max_fee_per_gas: max_fee,
            priority_fee_per_gas: 0,
            to: Some(fortiquo_types::Address::new([9u8; 20])),
            value,
            data: vec![],
            tx_kind: TransactionKind::Transfer,
            memo: None,
        };
        let msg = unsigned.serialize_for_signing().unwrap();
        let sig = kp.sign(&msg).unwrap();
        SignedTransaction::new(
            unsigned,
            PublicKeyBytes::new(kp.public_key().as_slice().to_vec()),
            sig,
            AlgorithmId::MlDsa44,
        )
    }

    #[test]
    fn test_mempool_rejects_duplicate_transaction() {
        // arrange
        let seed = b"dup test seed must be thirty two bytes!!";
        let kp = MlDsa44Keypair::from_seed(seed).unwrap();
        let mut state = InMemoryStateStore::new();
        let addr = kp.address().unwrap();
        state.set_account(addr, fortiquo_types::Account::new(1_000_000_000_000)).unwrap();
        state.commit().unwrap();
        let mut pool = Mempool::new(1);
        let tx = sample_tx(&kp, 0, 1, 21_000, 1, 0);

        // act
        pool.admit(tx.clone(), &state).unwrap();
        let err = pool.admit(tx, &state);

        // assert
        assert_eq!(err, Err(MempoolError::DuplicateTransaction));
    }

    #[test]
    fn test_mempool_select_orders_by_fee() {
        // arrange
        let seed_a = b"fee order test key a thirty two bytes!!";
        let seed_b = b"fee order test key b thirty two bytes!!";
        let kpa = MlDsa44Keypair::from_seed(seed_a).unwrap();
        let kpb = MlDsa44Keypair::from_seed(seed_b).unwrap();
        let mut state = InMemoryStateStore::new();
        state
            .set_account(kpa.address().unwrap(), fortiquo_types::Account::new(10u128.pow(18)))
            .unwrap();
        state
            .set_account(kpb.address().unwrap(), fortiquo_types::Account::new(10u128.pow(18)))
            .unwrap();
        state.commit().unwrap();
        let mut pool = Mempool::new(1);
        let low = sample_tx(&kpa, 0, 1, 21_000, 1, 0);
        let high = sample_tx(&kpb, 0, 1, 21_000, 100, 0);
        pool.admit(low, &state).unwrap();
        pool.admit(high, &state).unwrap();

        // act
        let sel = pool.select_transactions(u64::MAX, 10);

        // assert
        assert_eq!(sel.len(), 2);
        assert_eq!(sel[0].unsigned_tx.max_fee_per_gas, 100);
        assert_eq!(sel[1].unsigned_tx.max_fee_per_gas, 1);
    }
}
