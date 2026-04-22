use thiserror::Error;

/// Errors that can occur when working with blockchain types.
#[derive(Error, Debug)]
pub enum TypeError {
    #[error("Invalid address: {0}")]
    InvalidAddress(String),

    #[error("Invalid hash: {0}")]
    InvalidHash(String),

    #[error("Invalid transaction: {0}")]
    InvalidTransaction(String),

    #[error("Invalid block: {0}")]
    InvalidBlock(String),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] postcard::Error),

    #[error("Transaction size exceeds limit")]
    TransactionTooLarge,

    #[error("Block size exceeds limit")]
    BlockTooLarge,

    #[error("Gas limit exceeds maximum")]
    GasLimitExceeded,

    #[error("Invalid transaction kind")]
    InvalidTransactionKind,

    #[error("Invalid algorithm ID")]
    InvalidAlgorithmId,

    #[error("Invalid validator")]
    InvalidValidator,

    #[error("Insufficient balance")]
    InsufficientBalance,

    #[error("Invalid nonce")]
    InvalidNonce,
}
