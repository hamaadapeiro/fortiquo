//! Proof-of-History recorder and verifier (BLAKE3 hash chain).

use crate::ConsensusError;
use fortiquo_types::{Hash, PohEntry, TxHash, Validator, ValidatorId};

/// BLAKE3 PoH step: `hash(previous_hash || tick_le || tx_hash_0 || ... || tx_hash_n)`.
pub fn poh_step_hash(previous_hash: Hash, tick: u64, tx_hashes: &[TxHash]) -> Hash {
    let mut buf = Vec::with_capacity(32 + 8 + tx_hashes.len() * 32);
    buf.extend_from_slice(previous_hash.as_bytes());
    buf.extend_from_slice(&tick.to_le_bytes());
    for h in tx_hashes {
        buf.extend_from_slice(h.as_bytes());
    }
    Hash::compute(&buf)
}

/// Advances the PoH hash chain and emits [`PohEntry`] values.
#[derive(Clone, Debug)]
pub struct PohRecorder {
    current_hash: Hash,
    current_tick: u64,
    leader_id: ValidatorId,
}

impl PohRecorder {
    /// Start a recorder anchored at `genesis_hash`, emitting entries for `leader_id`.
    pub fn new(genesis_hash: Hash, leader: ValidatorId) -> Self {
        PohRecorder {
            current_hash: genesis_hash,
            current_tick: 0,
            leader_id: leader,
        }
    }

    /// Append a tick with no transactions.
    pub fn tick(&mut self) -> PohEntry {
        let previous_hash = self.current_hash;
        let tick_number = self.current_tick;
        let current_hash = poh_step_hash(previous_hash, tick_number, &[]);
        let entry = PohEntry {
            previous_hash,
            current_hash,
            tick_number,
            tx_hashes: vec![],
            leader_id: self.leader_id,
        };
        self.current_hash = current_hash;
        self.current_tick += 1;
        entry
    }

    /// Append a tick mixing in the given transaction hashes (order is significant).
    pub fn record_transactions(&mut self, tx_hashes: Vec<TxHash>) -> PohEntry {
        let previous_hash = self.current_hash;
        let tick_number = self.current_tick;
        let current_hash = poh_step_hash(previous_hash, tick_number, &tx_hashes);
        let entry = PohEntry {
            previous_hash,
            current_hash,
            tick_number,
            tx_hashes,
            leader_id: self.leader_id,
        };
        self.current_hash = current_hash;
        self.current_tick += 1;
        entry
    }

    pub fn current_hash(&self) -> Hash {
        self.current_hash
    }

    pub fn current_tick(&self) -> u64 {
        self.current_tick
    }

    pub fn leader_id(&self) -> ValidatorId {
        self.leader_id
    }
}

/// Verifies PoH entry sequences against the hash chain and leader schedule.
#[derive(Clone, Copy, Debug, Default)]
pub struct PohVerifier;

impl PohVerifier {
    /// Verify a contiguous PoH slice.
    ///
    /// `genesis_hash` is the hash **before** the first entry (genesis anchor or prior block state).
    pub fn verify_sequence(
        entries: &[PohEntry],
        genesis_hash: Hash,
        validators: &[Validator],
        ticks_per_slot: u64,
    ) -> Result<(), ConsensusError> {
        if validators.is_empty() {
            return Err(ConsensusError::EmptyValidatorSet);
        }
        if ticks_per_slot == 0 {
            return Err(ConsensusError::InvalidTicksPerSlot);
        }

        for (i, entry) in entries.iter().enumerate() {
            if i == 0 {
                if entry.previous_hash != genesis_hash {
                    return Err(ConsensusError::AnchorMismatch);
                }
            } else {
                let prev = &entries[i - 1];
                if entry.tick_number != prev.tick_number.saturating_add(1) {
                    return Err(ConsensusError::NonSequentialTicks);
                }
                if entry.previous_hash != prev.current_hash {
                    return Err(ConsensusError::HashChainBroken);
                }
            }

            let expected = poh_step_hash(
                entry.previous_hash,
                entry.tick_number,
                &entry.tx_hashes,
            );
            if entry.current_hash != expected {
                return Err(ConsensusError::HashMismatch);
            }

            let slot = entry.tick_number / ticks_per_slot;
            let expected_leader = &validators[slot as usize % validators.len()];
            if entry.leader_id != expected_leader.id {
                return Err(ConsensusError::UnauthorizedLeader);
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fortiquo_types::Validator;

    #[test]
    fn recorder_then_verifier_happy_path() {
        let genesis = Hash::new([9u8; 32]);
        let leader = ValidatorId::new([1u8; 32]);
        let validators = vec![Validator::new(leader)];

        let mut rec = PohRecorder::new(genesis, leader);
        let mut entries = vec![];
        entries.push(rec.tick());
        entries.push(rec.record_transactions(vec![TxHash::new(Hash::new([5u8; 32]))]));
        entries.push(rec.tick());

        PohVerifier::verify_sequence(&entries, genesis, &validators, 400).unwrap();
    }

    #[test]
    fn rejects_tampered_hash() {
        let genesis = Hash::zero();
        let leader = ValidatorId::new([2u8; 32]);
        let validators = vec![Validator::new(leader)];

        let mut rec = PohRecorder::new(genesis, leader);
        let mut e = rec.tick();
        e.current_hash = Hash::new([0xabu8; 32]);

        assert_eq!(
            PohVerifier::verify_sequence(&[e], genesis, &validators, 400),
            Err(ConsensusError::HashMismatch)
        );
    }

    #[test]
    fn rejects_wrong_leader() {
        let genesis = Hash::zero();
        let good = ValidatorId::new([3u8; 32]);
        let bad_leader = ValidatorId::new([4u8; 32]);
        let validators = vec![Validator::new(good), Validator::new(ValidatorId::new([5u8; 32]))];

        let mut rec = PohRecorder::new(genesis, bad_leader);
        let e = rec.tick();

        assert_eq!(
            PohVerifier::verify_sequence(&[e], genesis, &validators, 400),
            Err(ConsensusError::UnauthorizedLeader)
        );
    }

    #[test]
    fn rejects_broken_chain() {
        let genesis = Hash::zero();
        let leader = ValidatorId::new([6u8; 32]);
        let validators = vec![Validator::new(leader)];

        let mut rec = PohRecorder::new(genesis, leader);
        let a = rec.tick();
        let mut b = rec.tick();
        b.previous_hash = Hash::new([1u8; 32]); // not equal to a.current_hash

        assert_eq!(
            PohVerifier::verify_sequence(&[a, b], genesis, &validators, 400),
            Err(ConsensusError::HashChainBroken)
        );
    }
}
