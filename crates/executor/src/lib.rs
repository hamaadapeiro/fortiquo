//! Block execution pipeline: PoH verification, leader checks, and sequential EVM runs.

mod block;
mod error;

pub use block::{BlockExecutor, BlockExecutorConfig};
pub use error::BlockExecutionError;

use fortiquo_types::{Hash, Receipt};

/// Outputs from [`BlockExecutor::execute_block`].
#[derive(Clone, Debug)]
pub struct BlockExecutionResult {
    /// One receipt per executed transaction (same order as the block body).
    pub receipts: Vec<Receipt>,
    /// State root returned by [`fortiquo_state::StateStore::commit`].
    pub state_root: Hash,
    /// Sum of per-transaction gas used.
    pub gas_used: u64,
    /// Number of transactions processed.
    pub tx_count: usize,
}
