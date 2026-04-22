//! Persistent and in-memory state errors.

use thiserror::Error;

/// Errors from [`crate::StateStore`](super::StateStore) operations.
#[derive(Debug, Error)]
pub enum StateError {
    /// RocksDB I/O or open error.
    #[error("rocksdb: {0}")]
    RocksDb(String),

    /// Postcard serialization or deserialization failed.
    #[error("serialization: {0}")]
    Serialization(String),

    /// Requested resource was not present.
    #[error("not found: {0}")]
    NotFound(String),

    /// Internal invariant violated.
    #[error("internal: {0}")]
    Internal(String),
}

#[cfg(feature = "rocksdb")]
impl From<rocksdb::Error> for StateError {
    fn from(value: rocksdb::Error) -> Self {
        StateError::RocksDb(value.to_string())
    }
}

impl From<postcard::Error> for StateError {
    fn from(value: postcard::Error) -> Self {
        StateError::Serialization(format!("{value:?}"))
    }
}
