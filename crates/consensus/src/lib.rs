//! Proof-of-History consensus helpers: recorder, verifier, and leader schedule.
//!
//! PoH step hashing is BLAKE3 over `previous_hash || tick (little-endian u64) || tx_hashes...`
//! as described in `docs/poh-consensus.md`.

mod error;
mod leader_schedule;
mod poh;

pub use error::ConsensusError;
pub use leader_schedule::LeaderSchedule;
pub use poh::{poh_step_hash, PohRecorder, PohVerifier};
