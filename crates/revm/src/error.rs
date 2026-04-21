use thiserror::Error;

/// Errors that can occur during EVM execution.
#[derive(Error, Debug)]
pub enum ExecutionError {
    #[error("Insufficient gas: required {required}, available {available}")]
    OutOfGas { required: u64, available: u64 },

    #[error("Invalid contract bytecode")]
    InvalidBytecode,

    #[error("Contract execution reverted: {reason}")]
    ExecutionReverted { reason: String },

    #[error("Invalid state transition")]
    InvalidStateTransition,

    #[error("Account not found")]
    AccountNotFound,

    #[error("Insufficient balance")]
    InsufficientBalance,

    #[error("Invalid nonce")]
    InvalidNonce,

    #[error("Stack overflow")]
    StackOverflow,

    #[error("Stack underflow")]
    StackUnderflow,

    #[error("Memory access out of bounds")]
    MemoryAccessOutOfBounds,

    #[error("Invalid opcode")]
    InvalidOpcode,

    #[error("Execution timeout")]
    ExecutionTimeout,

    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("State database error: {0}")]
    StateError(String),

    #[error("Invalid transaction: {0}")]
    InvalidTransaction(String),
}

impl From<postcard::Error> for ExecutionError {
    fn from(err: postcard::Error) -> Self {
        ExecutionError::SerializationError(format!("{:?}", err))
    }
}

impl From<String> for ExecutionError {
    fn from(err: String) -> Self {
        ExecutionError::StateError(err)
    }
}

