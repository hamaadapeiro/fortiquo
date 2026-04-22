//! RocksDB-backed and in-memory [`StateStore`](store::StateStore) implementations.
//!
//! State roots are deterministic BLAKE3 aggregates over accounts, contract storage, and code
//! bodies (see [`state_root::compute_state_root`]).
//!
//! Compile with `--features rocksdb` to enable [`RocksDbStateStore`] (requires a working
//! `librocksdb-sys` native build).

mod error;
mod in_memory;
#[cfg(feature = "rocksdb")]
mod rocksdb_store;
mod state_root;
mod store;

pub use error::StateError;
pub use in_memory::InMemoryStateStore;
#[cfg(feature = "rocksdb")]
pub use rocksdb_store::{
    RocksDbStateStore, CF_ACCOUNTS, CF_BLOCKS, CF_CODE, CF_METADATA, CF_RECEIPTS, CF_STORAGE,
};
pub use state_root::compute_state_root;
pub use store::StateStore;
