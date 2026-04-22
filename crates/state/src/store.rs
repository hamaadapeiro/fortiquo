//! [`StateStore`] trait — shared interface for RocksDB and in-memory implementations.

use crate::StateError;
use fortiquo_types::{Account, Address, Block, Hash, Receipt, TxHash};

/// Persistent or ephemeral blockchain state (accounts, storage, blocks, receipts).
///
/// Implementations buffer mutations until [`StateStore::commit`](StateStore::commit) publishes a
/// new BLAKE3 state root, or [`StateStore::rollback`](StateStore::rollback) discards pending work.
pub trait StateStore: Send + Sync {
    /// Load an account; returns [`Account::empty`] if never written.
    fn get_account(&self, addr: &Address) -> Result<Account, StateError>;

    /// Upsert an account.
    fn set_account(&mut self, addr: Address, account: Account) -> Result<(), StateError>;

    /// Read a 32-byte storage slot (`Hash::zero` if unset).
    fn get_storage(&self, addr: &Address, slot: &Hash) -> Result<Hash, StateError>;

    /// Write a 32-byte EVM storage word.
    fn set_storage(&mut self, addr: Address, slot: Hash, value: Hash) -> Result<(), StateError>;

    /// Contract bytecode for `addr` (empty vector if none).
    fn get_contract_code(&self, addr: &Address) -> Result<Vec<u8>, StateError>;

    /// Store bytecode keyed by contract address.
    fn set_contract_code(&mut self, addr: Address, code: Vec<u8>) -> Result<(), StateError>;

    /// Block by height.
    fn get_block(&self, number: u64) -> Result<Block, StateError>;

    /// Persist a full block.
    fn set_block(&mut self, block: Block) -> Result<(), StateError>;

    /// Receipt for a committed transaction.
    fn get_receipt(&self, tx_hash: &TxHash) -> Result<Receipt, StateError>;

    /// Store an execution receipt.
    fn set_receipt(&mut self, receipt: Receipt) -> Result<(), StateError>;

    /// Finalize buffered writes and return the new BLAKE3 state root.
    fn commit(&mut self) -> Result<Hash, StateError>;

    /// Drop uncommitted writes since the last successful commit.
    fn rollback(&mut self) -> Result<(), StateError>;
}
