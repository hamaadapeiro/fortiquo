//! Mempool admission and selection errors.

use thiserror::Error;

/// Errors returned while admitting or removing transactions.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum MempoolError {
    #[error("invalid signature or public key")]
    InvalidSignature,

    #[error("transaction too large (serialized bytes)")]
    TransactionTooLarge,

    #[error("duplicate transaction hash")]
    DuplicateTransaction,

    #[error("pending transaction already uses this sender nonce")]
    NonceConflict,

    #[error("invalid nonce for current account state")]
    InvalidNonce,

    #[error("insufficient balance for gas and value")]
    InsufficientBalance,

    #[error("chain id mismatch")]
    InvalidChainId,

    #[error("state lookup failed: {0}")]
    State(String),
}
