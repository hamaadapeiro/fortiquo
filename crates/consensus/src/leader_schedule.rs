use fortiquo_types::Validator;

/// Round-robin leader schedule over a fixed validator ordering.
#[derive(Clone, Debug)]
pub struct LeaderSchedule {
    pub validators: Vec<Validator>,
}

impl LeaderSchedule {
    /// Create a schedule from the given ordering (slot `s` uses index `s % len`).
    pub fn new(validators: Vec<Validator>) -> Self {
        LeaderSchedule { validators }
    }

    /// Leader for a slot index (0-based).
    ///
    /// # Panics
    /// Panics if the validator list is empty.
    pub fn leader_for_slot(&self, slot: u64) -> &Validator {
        let idx = slot as usize % self.validators.len();
        &self.validators[idx]
    }

    /// Leader for a PoH tick given ticks per slot.
    ///
    /// # Panics
    /// Panics if `ticks_per_slot == 0` or the validator list is empty.
    pub fn leader_for_tick(&self, tick: u64, ticks_per_slot: u64) -> &Validator {
        assert!(ticks_per_slot > 0, "ticks_per_slot must be non-zero");
        let slot = tick / ticks_per_slot;
        self.leader_for_slot(slot)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fortiquo_types::ValidatorId;

    #[test]
    fn round_robin_by_slot() {
        let v0 = Validator::new(ValidatorId::new([1u8; 32]));
        let v1 = Validator::new(ValidatorId::new([2u8; 32]));
        let schedule = LeaderSchedule::new(vec![v0.clone(), v1.clone()]);

        assert_eq!(schedule.leader_for_slot(0).id, v0.id);
        assert_eq!(schedule.leader_for_slot(1).id, v1.id);
        assert_eq!(schedule.leader_for_slot(2).id, v0.id);
    }

    #[test]
    fn leader_for_tick_maps_to_slot() {
        let v0 = Validator::new(ValidatorId::new([10u8; 32]));
        let v1 = Validator::new(ValidatorId::new([20u8; 32]));
        let schedule = LeaderSchedule::new(vec![v0, v1]);

        assert_eq!(schedule.leader_for_tick(0, 400).id, ValidatorId::new([10u8; 32]));
        assert_eq!(schedule.leader_for_tick(399, 400).id, ValidatorId::new([10u8; 32]));
        assert_eq!(schedule.leader_for_tick(400, 400).id, ValidatorId::new([20u8; 32]));
    }
}
