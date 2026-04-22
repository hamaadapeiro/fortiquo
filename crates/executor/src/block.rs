//! [`BlockExecutor`] — PoH verification plus sequential signed transaction execution.

use crate::BlockExecutionError;
use crate::BlockExecutionResult;
use fortiquo_consensus::PohVerifier;
use fortiquo_revm::Executor;
use fortiquo_state::StateStore;
use fortiquo_types::{Address, Block, Hash, Receipt, Validator};

/// Configuration for [`BlockExecutor`].
#[derive(Clone, Debug)]
pub struct BlockExecutorConfig {
    /// PoH ticks per slot (passed to [`PohVerifier::verify_sequence`]).
    pub ticks_per_slot: u64,
}

impl Default for BlockExecutorConfig {
    fn default() -> Self {
        BlockExecutorConfig {
            ticks_per_slot: 400,
        }
    }
}

/// Runs PoH checks and feeds transactions into the [`Executor`] while persisting to [`StateStore`].
pub struct BlockExecutor {
    /// Persistent world state and receipts.
    pub state: Box<dyn StateStore>,
    /// Per-block EVM + ML-DSA execution engine.
    pub evm: Executor,
    pub config: BlockExecutorConfig,
}

impl BlockExecutor {
    /// Create a new pipeline with the given store, EVM executor, and PoH settings.
    pub fn new(
        state: Box<dyn StateStore>,
        evm: Executor,
        config: BlockExecutorConfig,
    ) -> Self {
        BlockExecutor { state, evm, config }
    }

    /// Verify PoH, leader, then execute each transaction in order.
    pub fn execute_block(
        &mut self,
        block: &Block,
        validators: &[Validator],
    ) -> Result<BlockExecutionResult, BlockExecutionError> {
        let anchor = block.header.poh_start_hash;
        PohVerifier::verify_sequence(
            &block.body.poh_entries,
            anchor,
            validators,
            self.config.ticks_per_slot,
        )
        .map_err(|e| BlockExecutionError::Consensus(e.to_string()))?;

        if validators.is_empty() {
            return Err(BlockExecutionError::InvalidLeader);
        }
        let slot = block.header.poh_start_tick / self.config.ticks_per_slot;
        let expected = &validators[slot as usize % validators.len()];
        if block.header.leader_id != expected.id {
            return Err(BlockExecutionError::InvalidLeader);
        }

        let block_hash = block.hash();
        let mut receipts = Vec::new();
        let mut gas_used = 0u64;

        for (idx, tx) in block.body.signed_transactions.iter().enumerate() {
            let sender = fortiquo_revm::Executor::verify_and_derive_sender(tx)
                .map_err(|e| BlockExecutionError::Execution(e.to_string()))?;

            self.hydrate_accounts_for_tx(sender, tx.unsigned_tx.to)?;

            let result = self
                .evm
                .execute_signed_for_sender(tx, sender)
                .map_err(|e| BlockExecutionError::Execution(e.to_string()))?;

            self.apply_state_changes_to_store(&result.state_changes)?;

            gas_used = gas_used.saturating_add(result.gas_used);

            let mut receipt = self.evm.create_receipt(
                tx.hash(),
                block.header.number,
                idx as u32,
                &result,
            );
            receipt.block_hash = block_hash;
            self.state
                .set_receipt(receipt.clone())
                .map_err(|e| BlockExecutionError::State(e.to_string()))?;
            receipts.push(receipt);
        }

        let state_root = self
            .state
            .commit()
            .map_err(|e| BlockExecutionError::State(e.to_string()))?;

        Ok(BlockExecutionResult {
            receipts,
            state_root,
            gas_used,
            tx_count: block.body.signed_transactions.len(),
        })
    }

    fn hydrate_accounts_for_tx(
        &mut self,
        sender: Address,
        to: Option<Address>,
    ) -> Result<(), BlockExecutionError> {
        let s = self
            .state
            .get_account(&sender)
            .map_err(|e| BlockExecutionError::State(e.to_string()))?;
        self.evm.state_mut().set_account(sender, s);
        if let Some(to) = to {
            let t = self
                .state
                .get_account(&to)
                .map_err(|e| BlockExecutionError::State(e.to_string()))?;
            self.evm.state_mut().set_account(to, t);
        }
        Ok(())
    }

    fn apply_state_changes_to_store(
        &mut self,
        changes: &fortiquo_revm::state::StateChanges,
    ) -> Result<(), BlockExecutionError> {
        use fortiquo_types::Hash;
        for (addr, acc) in &changes.modified_accounts {
            self.state
                .set_account(*addr, acc.clone())
                .map_err(|e| BlockExecutionError::State(e.to_string()))?;
        }
        for addr in &changes.deleted_accounts {
            self.state
                .set_account(*addr, fortiquo_types::Account::empty())
                .map_err(|e| BlockExecutionError::State(e.to_string()))?;
        }
        for (addr, slots) in &changes.storage_changes {
            for (slot, v) in slots {
                let h = if v.len() == 32 {
                    let mut a = [0u8; 32];
                    a.copy_from_slice(v);
                    Hash::new(a)
                } else if v.is_empty() {
                    Hash::zero()
                } else {
                    Hash::compute(v)
                };
                self.state
                    .set_storage(*addr, *slot, h)
                    .map_err(|e| BlockExecutionError::State(e.to_string()))?;
            }
        }
        Ok(())
    }
}
