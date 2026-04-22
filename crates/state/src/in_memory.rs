//! In-memory [`StateStore`](crate::StateStore) wrapping [`fortiquo_revm::StateManager`] for tests.

use crate::state_root::compute_state_root;
use crate::store::StateStore;
use crate::StateError;
use fortiquo_revm::StateManager;
use fortiquo_types::{Account, Address, Block, Hash, Receipt, TxHash};
use std::collections::{BTreeMap, HashMap};

/// Snapshot of all mutable layers for [`InMemoryStateStore::rollback`].
#[derive(Clone)]
struct InMemorySnapshot {
    svm: StateManager,
    code: HashMap<Address, Vec<u8>>,
    blocks: HashMap<u64, Block>,
    receipts: HashMap<TxHash, Receipt>,
}

/// [`StateStore`](crate::StateStore) backed by an in-memory [`StateManager`] plus auxiliary maps.
///
/// Used in unit tests and local nodes; clones a full snapshot on the first write after a commit
/// so [`StateStore::rollback`](crate::StateStore::rollback) restores the prior committed view.
pub struct InMemoryStateStore {
    svm: StateManager,
    code: HashMap<Address, Vec<u8>>,
    blocks: HashMap<u64, Block>,
    receipts: HashMap<TxHash, Receipt>,
    rollback: Option<InMemorySnapshot>,
}

impl InMemoryStateStore {
    /// Empty store.
    pub fn new() -> Self {
        InMemoryStateStore {
            svm: StateManager::new(),
            code: HashMap::new(),
            blocks: HashMap::new(),
            receipts: HashMap::new(),
            rollback: None,
        }
    }

    fn ensure_rollback_point(&mut self) {
        if self.rollback.is_none() {
            self.rollback = Some(InMemorySnapshot {
                svm: self.svm.clone(),
                code: self.code.clone(),
                blocks: self.blocks.clone(),
                receipts: self.receipts.clone(),
            });
        }
    }

    fn build_root(&self) -> Hash {
        let mut accounts = BTreeMap::new();
        for (addr, acc) in self.svm.get_all_accounts() {
            accounts.insert(*addr, acc.clone());
        }
        let mut storage = BTreeMap::new();
        for (addr, slot, v) in self.svm.storage_entries() {
            storage.insert((addr, slot), vec_to_storage_hash(&v));
        }
        let mut code_hashes = BTreeMap::new();
        for (addr, bytes) in &self.code {
            code_hashes.insert(*addr, Hash::compute(bytes));
        }
        compute_state_root(&accounts, &storage, &code_hashes)
    }
}

impl Default for InMemoryStateStore {
    fn default() -> Self {
        Self::new()
    }
}

fn vec_to_storage_hash(v: &[u8]) -> Hash {
    if v.len() == 32 {
        let mut a = [0u8; 32];
        a.copy_from_slice(v);
        Hash::new(a)
    } else if v.is_empty() {
        Hash::zero()
    } else {
        Hash::compute(v)
    }
}

impl StateStore for InMemoryStateStore {
    fn get_account(&self, addr: &Address) -> Result<Account, StateError> {
        Ok(self.svm.get_account(addr))
    }

    fn set_account(&mut self, addr: Address, account: Account) -> Result<(), StateError> {
        self.ensure_rollback_point();
        self.svm.set_account(addr, account);
        Ok(())
    }

    fn get_storage(&self, addr: &Address, slot: &Hash) -> Result<Hash, StateError> {
        let v = self.svm.get_storage(addr, slot);
        Ok(vec_to_storage_hash(&v))
    }

    fn set_storage(&mut self, addr: Address, slot: Hash, value: Hash) -> Result<(), StateError> {
        self.ensure_rollback_point();
        self.svm
            .set_storage(addr, slot, value.as_bytes().to_vec());
        Ok(())
    }

    fn get_contract_code(&self, addr: &Address) -> Result<Vec<u8>, StateError> {
        Ok(self.code.get(addr).cloned().unwrap_or_default())
    }

    fn set_contract_code(&mut self, addr: Address, code: Vec<u8>) -> Result<(), StateError> {
        self.ensure_rollback_point();
        self.code.insert(addr, code);
        Ok(())
    }

    fn get_block(&self, number: u64) -> Result<Block, StateError> {
        self.blocks
            .get(&number)
            .cloned()
            .ok_or_else(|| StateError::NotFound(format!("block {number}")))
    }

    fn set_block(&mut self, block: Block) -> Result<(), StateError> {
        self.ensure_rollback_point();
        let n = block.header.number;
        self.blocks.insert(n, block);
        Ok(())
    }

    fn get_receipt(&self, tx_hash: &TxHash) -> Result<Receipt, StateError> {
        self.receipts
            .get(tx_hash)
            .cloned()
            .ok_or_else(|| StateError::NotFound(format!("receipt {}", tx_hash)))
    }

    fn set_receipt(&mut self, receipt: Receipt) -> Result<(), StateError> {
        self.ensure_rollback_point();
        self.receipts.insert(receipt.tx_hash, receipt);
        Ok(())
    }

    fn commit(&mut self) -> Result<Hash, StateError> {
        let root = self.build_root();
        self.rollback = None;
        Ok(root)
    }

    fn rollback(&mut self) -> Result<(), StateError> {
        if let Some(snap) = self.rollback.take() {
            self.svm = snap.svm;
            self.code = snap.code;
            self.blocks = snap.blocks;
            self.receipts = snap.receipts;
        }
        Ok(())
    }
}

/// Expose the embedded [`StateManager`] for EVM tests that need direct access.
impl InMemoryStateStore {
    /// Borrow the underlying account/storage manager.
    pub fn state_manager(&self) -> &StateManager {
        &self.svm
    }

    /// Mutable access to the underlying account/storage manager (does not update rollback snapshot).
    pub fn state_manager_mut(&mut self) -> &mut StateManager {
        &mut self.svm
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fortiquo_types::BlockBody;

    #[test]
    fn test_in_memory_roundtrip_account_postcard_stable() {
        // arrange
        let mut store = InMemoryStateStore::new();
        let addr = Address::new([7u8; 20]);
        let acc = Account::new(12345);

        // act
        store.set_account(addr, acc.clone()).unwrap();
        let got = store.get_account(&addr).unwrap();

        // assert
        assert_eq!(got.balance, acc.balance);
    }

    #[test]
    fn test_in_memory_rollback_restores_account() {
        // arrange
        let mut store = InMemoryStateStore::new();
        let addr = Address::new([1u8; 20]);
        store.set_account(addr, Account::new(10)).unwrap();
        store.commit().unwrap();

        // act
        store.set_account(addr, Account::new(99)).unwrap();
        store.rollback().unwrap();

        // assert
        assert_eq!(store.get_account(&addr).unwrap().balance, 10);
    }

    #[test]
    fn test_in_memory_commit_clears_pending_rollback() {
        // arrange
        let mut store = InMemoryStateStore::new();
        let addr = Address::new([2u8; 20]);
        store.set_account(addr, Account::new(1)).unwrap();
        store.commit().unwrap();
        store.set_account(addr, Account::new(2)).unwrap();

        // act
        store.commit().unwrap();
        store.rollback().unwrap();

        // assert — second rollback is no-op; balance stays at 2
        assert_eq!(store.get_account(&addr).unwrap().balance, 2);
    }

    #[test]
    fn test_in_memory_block_roundtrip() {
        // arrange
        use fortiquo_types::{BlockHash, BlockHeader, ValidatorId};

        let mut store = InMemoryStateStore::new();
        let header = BlockHeader {
            number: 5,
            parent_hash: BlockHash::new(Hash::zero()),
            state_root: Hash::zero(),
            tx_root: Hash::zero(),
            receipts_root: Hash::zero(),
            poh_start_hash: Hash::zero(),
            poh_end_hash: Hash::zero(),
            poh_start_tick: 0,
            poh_end_tick: 0,
            leader_id: ValidatorId::new([0u8; 32]),
            timestamp: 0,
            gas_used: 0,
            gas_limit: 0,
        };
        let body = BlockBody {
            poh_entries: vec![],
            signed_transactions: vec![],
        };
        let block = Block::new(header, body);

        // act
        store.set_block(block.clone()).unwrap();
        let loaded = store.get_block(5).unwrap();

        // assert
        assert_eq!(loaded.header.number, block.header.number);
    }
}
