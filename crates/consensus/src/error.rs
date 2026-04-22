use thiserror::Error;

/// Errors returned by consensus verification and schedule helpers.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum ConsensusError {
    /// No validators were supplied where at least one is required.
    #[error("validator set is empty")]
    EmptyValidatorSet,

    /// `ticks_per_slot` must be non-zero for leader derivation and PoH verification.
    #[error("ticks_per_slot must be greater than zero")]
    InvalidTicksPerSlot,

    /// The first entry's `previous_hash` does not match the expected chain anchor.
    #[error("first PoH entry previous_hash does not match the expected anchor")]
    AnchorMismatch,

    /// `tick_number` is not strictly increasing by one across adjacent entries.
    #[error("non-sequential PoH tick numbers")]
    NonSequentialTicks,

    /// `previous_hash` does not match the prior entry's `current_hash`.
    #[error("broken PoH hash chain linkage")]
    HashChainBroken,

    /// Recomputed BLAKE3 step does not match `current_hash`.
    #[error("PoH current_hash does not match recomputed hash")]
    HashMismatch,

    /// Entry `leader_id` is not the scheduled leader for this tick.
    #[error("PoH entry leader_id does not match leader schedule for tick")]
    UnauthorizedLeader,
}
