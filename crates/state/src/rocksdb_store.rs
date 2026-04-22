//! RocksDB-backed [`StateStore`](crate::StateStore) with column families per subsystem.

use crate::state_root::compute_state_root;
use crate::store::StateStore;
use crate::StateError;
use fortiquo_types::{Account, Address, Block, Hash, Receipt, TxHash};
use rocksdb::{ColumnFamilyDescriptor, IteratorMode, Options, DB};
use std::collections::{BTreeMap, HashMap};
use std::path::Path;
use std::sync::Arc;

/// Column family names (stable on-disk identifiers).
pub const CF_ACCOUNTS: &str = "accounts";
pub const CF_STORAGE: &str = "storage";
pub const CF_CODE: &str = "code";
pub const CF_BLOCKS: &str = "blocks";
pub const CF_RECEIPTS: &str = "receipts";
pub const CF_METADATA: &str = "metadata";

type OverlayKey = (&'static str, Vec<u8>);

#[derive(Clone)]
enum PendingWrite {
    Put(Vec<u8>),
    Delete,
}

/// Production [`StateStore`](crate::StateStore) using RocksDB column families.
///
/// Mutations are buffered until [`StateStore::commit`](crate::StateStore::commit) writes a batch;
/// [`StateStore::rollback`](crate::StateStore::rollback) discards the overlay.
pub struct RocksDbStateStore {
    db: Arc<DB>,
    overlay: HashMap<OverlayKey, PendingWrite>,
}

impl RocksDbStateStore {
    /// Open or create a database at `path` with all required column families.
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, StateError> {
        let mut opts = Options::default();
        opts.create_if_missing(true);
        opts.create_missing_column_families(true);

        let cfs = vec![
            ColumnFamilyDescriptor::new(CF_ACCOUNTS, Options::default()),
            ColumnFamilyDescriptor::new(CF_STORAGE, Options::default()),
            ColumnFamilyDescriptor::new(CF_CODE, Options::default()),
            ColumnFamilyDescriptor::new(CF_BLOCKS, Options::default()),
            ColumnFamilyDescriptor::new(CF_RECEIPTS, Options::default()),
            ColumnFamilyDescriptor::new(CF_METADATA, Options::default()),
        ];

        let db = DB::open_cf_descriptors(&opts, path, cfs).map_err(|e| StateError::RocksDb(e.to_string()))?;
        Ok(RocksDbStateStore {
            db: Arc::new(db),
            overlay: HashMap::new(),
        })
    }

    fn cf(&self, name: &'static str) -> Result<&rocksdb::ColumnFamily, StateError> {
        self.db
            .cf_handle(name)
            .ok_or_else(|| StateError::Internal(format!("missing column family {name}")))
    }

    fn get_raw(&self, cf: &'static str, key: &[u8]) -> Result<Option<Vec<u8>>, StateError> {
        let kfull: OverlayKey = (cf, key.to_vec());
        match self.overlay.get(&kfull) {
            Some(PendingWrite::Delete) => return Ok(None),
            Some(PendingWrite::Put(b)) => return Ok(Some(b.clone())),
            None => {}
        }
        Ok(self.db.get_cf(self.cf(cf)?, key)?)
    }

    fn put_overlay(&mut self, cf: &'static str, key: Vec<u8>, val: Vec<u8>) {
        self.overlay.insert((cf, key), PendingWrite::Put(val));
    }

    fn storage_key(addr: &Address, slot: &Hash) -> Vec<u8> {
        let mut k = Vec::with_capacity(52);
        k.extend_from_slice(addr.as_bytes());
        k.extend_from_slice(slot.as_bytes());
        k
    }

    fn block_key(number: u64) -> Vec<u8> {
        number.to_le_bytes().to_vec()
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

    fn scan_state_for_root(&self) -> Result<Hash, StateError> {
        let mut accounts = BTreeMap::new();
        {
            let cf = self.cf(CF_ACCOUNTS)?;
            let iter = self.db.iterator_cf(cf, IteratorMode::Start);
            for item in iter {
                let (k, v) = item.map_err(|e| StateError::RocksDb(e.to_string()))?;
                if k.len() != 20 {
                    continue;
                }
                let addr = Address::try_from_slice(&k).ok_or_else(|| {
                    StateError::Internal("account key length".into())
                })?;
                let acc: Account = postcard::from_bytes(&v)?;
                accounts.insert(addr, acc);
            }
        }

        let mut storage = BTreeMap::new();
        {
            let cf = self.cf(CF_STORAGE)?;
            let iter = self.db.iterator_cf(cf, IteratorMode::Start);
            for item in iter {
                let (k, v) = item.map_err(|e| StateError::RocksDb(e.to_string()))?;
                if k.len() != 52 {
                    continue;
                }
                let addr = Address::try_from_slice(&k[..20]).ok_or_else(|| {
                    StateError::Internal("storage address".into())
                })?;
                let slot = Hash::try_from_slice(&k[20..52])
                    .ok_or_else(|| StateError::Internal("storage slot".into()))?;
                storage.insert((addr, slot), Self::vec_to_storage_hash(&v));
            }
        }

        let mut code_hashes = BTreeMap::new();
        {
            let cf = self.cf(CF_CODE)?;
            let iter = self.db.iterator_cf(cf, IteratorMode::Start);
            for item in iter {
                let (k, v) = item.map_err(|e| StateError::RocksDb(e.to_string()))?;
                if k.len() != 20 {
                    continue;
                }
                let addr = Address::try_from_slice(&k).ok_or_else(|| {
                    StateError::Internal("code address".into())
                })?;
                code_hashes.insert(addr, Hash::compute(&v));
            }
        }

        Ok(compute_state_root(&accounts, &storage, &code_hashes))
    }

    fn apply_overlay_to_db(&mut self) -> Result<(), StateError> {
        let mut batch = rocksdb::WriteBatch::default();
        for ((cf, key), val) in self.overlay.drain() {
            let h = self.cf(cf)?;
            match val {
                PendingWrite::Put(bytes) => batch.put_cf(h, &key, &bytes),
                PendingWrite::Delete => batch.delete_cf(h, &key),
            }
        }
        self.db
            .write(batch)
            .map_err(|e| StateError::RocksDb(e.to_string()))?;
        Ok(())
    }
}

impl StateStore for RocksDbStateStore {
    fn get_account(&self, addr: &Address) -> Result<Account, StateError> {
        let key = addr.as_bytes().to_vec();
        match self.get_raw(CF_ACCOUNTS, &key)? {
            None => Ok(Account::empty()),
            Some(bytes) => Ok(postcard::from_bytes(&bytes)?),
        }
    }

    fn set_account(&mut self, addr: Address, account: Account) -> Result<(), StateError> {
        let bytes = postcard::to_allocvec(&account)?;
        self.put_overlay(CF_ACCOUNTS, addr.as_bytes().to_vec(), bytes);
        Ok(())
    }

    fn get_storage(&self, addr: &Address, slot: &Hash) -> Result<Hash, StateError> {
        let key = Self::storage_key(addr, slot);
        match self.get_raw(CF_STORAGE, &key)? {
            None => Ok(Hash::zero()),
            Some(v) => Ok(Self::vec_to_storage_hash(&v)),
        }
    }

    fn set_storage(&mut self, addr: Address, slot: Hash, value: Hash) -> Result<(), StateError> {
        let key = Self::storage_key(&addr, &slot);
        self.put_overlay(CF_STORAGE, key, value.as_bytes().to_vec());
        Ok(())
    }

    fn get_contract_code(&self, addr: &Address) -> Result<Vec<u8>, StateError> {
        let key = addr.as_bytes().to_vec();
        match self.get_raw(CF_CODE, &key)? {
            None => Ok(vec![]),
            Some(v) => Ok(v),
        }
    }

    fn set_contract_code(&mut self, addr: Address, code: Vec<u8>) -> Result<(), StateError> {
        self.put_overlay(CF_CODE, addr.as_bytes().to_vec(), code);
        Ok(())
    }

    fn get_block(&self, number: u64) -> Result<Block, StateError> {
        let key = Self::block_key(number);
        match self.get_raw(CF_BLOCKS, &key)? {
            None => Err(StateError::NotFound(format!("block {number}"))),
            Some(bytes) => Ok(postcard::from_bytes(&bytes)?),
        }
    }

    fn set_block(&mut self, block: Block) -> Result<(), StateError> {
        let n = block.header.number;
        let bytes = postcard::to_allocvec(&block)?;
        self.put_overlay(CF_BLOCKS, Self::block_key(n), bytes);
        Ok(())
    }

    fn get_receipt(&self, tx_hash: &TxHash) -> Result<Receipt, StateError> {
        let key = tx_hash.as_bytes().to_vec();
        match self.get_raw(CF_RECEIPTS, &key)? {
            None => Err(StateError::NotFound(format!("receipt {}", tx_hash))),
            Some(bytes) => Ok(postcard::from_bytes(&bytes)?),
        }
    }

    fn set_receipt(&mut self, receipt: Receipt) -> Result<(), StateError> {
        let key = receipt.tx_hash.as_bytes().to_vec();
        let bytes = postcard::to_allocvec(&receipt)?;
        self.put_overlay(CF_RECEIPTS, key, bytes);
        Ok(())
    }

    fn commit(&mut self) -> Result<Hash, StateError> {
        self.apply_overlay_to_db()?;
        self.scan_state_for_root()
    }

    fn rollback(&mut self) -> Result<(), StateError> {
        self.overlay.clear();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fortiquo_types::{BlockBody, BlockHash, BlockHeader, ValidatorId};

    #[test]
    fn test_rocksdb_state_store_roundtrip_account_postcard() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = RocksDbStateStore::open(dir.path()).unwrap();
        let addr = Address::new([3u8; 20]);
        let acc = Account::new(999);

        store.set_account(addr, acc.clone()).unwrap();
        store.commit().unwrap();

        let store2 = RocksDbStateStore::open(dir.path()).unwrap();
        let got = store2.get_account(&addr).unwrap();
        assert_eq!(got.balance, 999);
    }

    #[test]
    fn test_rocksdb_rollback_drops_overlay() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = RocksDbStateStore::open(dir.path()).unwrap();
        let addr = Address::new([4u8; 20]);
        store.set_account(addr, Account::new(1)).unwrap();
        store.commit().unwrap();

        store.set_account(addr, Account::new(2)).unwrap();
        store.rollback().unwrap();

        let store2 = RocksDbStateStore::open(dir.path()).unwrap();
        assert_eq!(store2.get_account(&addr).unwrap().balance, 1);
    }

    #[test]
    fn test_rocksdb_block_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = RocksDbStateStore::open(dir.path()).unwrap();
        let header = BlockHeader {
            number: 1,
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
        let block = Block::new(header, BlockBody {
            poh_entries: vec![],
            signed_transactions: vec![],
        });
        store.set_block(block.clone()).unwrap();
        store.commit().unwrap();

        let store2 = RocksDbStateStore::open(dir.path()).unwrap();
        let got = store2.get_block(1).unwrap();
        assert_eq!(got.header.number, 1);
    }
}
