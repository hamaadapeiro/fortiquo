//! Shared JSON-RPC context (chain state, caches, validator set).

use fortiquo_consensus::LeaderSchedule;
use fortiquo_revm::Executor;
use fortiquo_state::StateStore;
use fortiquo_types::{Block, PohEntry, SignedTransaction, TxHash, Validator, ValidatorId};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Thread-safe context backing JSON-RPC method handlers.
pub struct ChainContext {
    /// Network id used when interpreting transactions.
    pub chain_id: u64,
    /// Canonical persistent state.
    pub state: Arc<Mutex<Box<dyn StateStore + Send>>>,
    /// Transactions submitted via `chain_sendRawTransaction` (for lookup before finalization).
    pub tx_cache: Arc<Mutex<HashMap<TxHash, SignedTransaction>>>,
    /// Optional PoH index (filled by the node when syncing / producing).
    pub poh_cache: Arc<Mutex<HashMap<u64, PohEntry>>>,
    /// Active validator set for leader schedule queries.
    pub validators: Arc<Vec<Validator>>,
    /// EVM executor clone for `chain_estimateGas` (isolated from the node’s hot path).
    pub executor: Arc<Mutex<Executor>>,
}

impl ChainContext {
    /// Build a new context with empty caches.
    pub fn new(
        chain_id: u64,
        state: Arc<Mutex<Box<dyn StateStore + Send>>>,
        validators: Arc<Vec<Validator>>,
        executor: Arc<Mutex<Executor>>,
    ) -> Self {
        ChainContext {
            chain_id,
            state,
            tx_cache: Arc::new(Mutex::new(HashMap::new())),
            poh_cache: Arc::new(Mutex::new(HashMap::new())),
            validators,
            executor,
        }
    }

    /// Insert or replace a PoH entry for `chain_getPohEntry`.
    pub fn insert_poh_entry(&self, tick: u64, entry: PohEntry) {
        let mut g = self.poh_cache.lock().expect("poh cache lock");
        g.insert(tick, entry);
    }
}

/// Build [`LeaderSchedule`] round-robin view over the configured validators.
pub fn leader_schedule(ctx: &ChainContext) -> LeaderSchedule {
    LeaderSchedule::new(ctx.validators.as_ref().clone())
}
