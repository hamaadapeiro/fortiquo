//! Block execution errors.

use thiserror::Error;

/// Errors while executing an entire block.
#[derive(Debug, Error)]
pub enum BlockExecutionError {
    #[error("consensus: {0}")]
    Consensus(String),

    #[error("invalid block leader")]
    InvalidLeader,

    #[error("execution: {0}")]
    Execution(String),

    #[error("state: {0}")]
    State(String),

    #[error("serialization: {0}")]
    Serialization(String),
}
