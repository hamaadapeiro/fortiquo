//! Deterministic BLAKE3 state root from accounts, contract storage, and bytecode blobs.

use fortiquo_types::{Account, Address, Hash};
use std::collections::BTreeMap;

/// Compute a state root from canonical ordered maps (used by stores after commit).
pub fn compute_state_root(
    accounts: &BTreeMap<Address, Account>,
    storage: &BTreeMap<(Address, Hash), Hash>,
    code_hashes: &BTreeMap<Address, Hash>,
) -> Hash {
    let mut h = blake3::Hasher::new();
    h.update(b"fortiquo-state-root-v1\0accounts");
    for (addr, acc) in accounts {
        h.update(addr.as_bytes());
        let bytes = postcard::to_allocvec(acc).expect("account encode");
        h.update(&bytes);
    }
    h.update(b"\0storage");
    for ((addr, slot), val) in storage {
        h.update(addr.as_bytes());
        h.update(slot.as_bytes());
        h.update(val.as_bytes());
    }
    h.update(b"\0code");
    for (addr, ch) in code_hashes {
        h.update(addr.as_bytes());
        h.update(ch.as_bytes());
    }
    let out = h.finalize();
    let mut arr = [0u8; 32];
    arr.copy_from_slice(out.as_bytes());
    Hash::new(arr)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_state_root_deterministic() {
        let mut a = BTreeMap::new();
        a.insert(Address::new([1u8; 20]), Account::new(100));
        let r1 = compute_state_root(&a, &BTreeMap::new(), &BTreeMap::new());
        let r2 = compute_state_root(&a, &BTreeMap::new(), &BTreeMap::new());
        assert_eq!(r1, r2);
    }

    #[test]
    fn test_state_root_order_independent_insertion() {
        let mut a = BTreeMap::new();
        a.insert(Address::new([2u8; 20]), Account::new(1));
        a.insert(Address::new([1u8; 20]), Account::new(2));
        let r_a = compute_state_root(&a, &BTreeMap::new(), &BTreeMap::new());

        let mut b = BTreeMap::new();
        b.insert(Address::new([1u8; 20]), Account::new(2));
        b.insert(Address::new([2u8; 20]), Account::new(1));
        let r_b = compute_state_root(&b, &BTreeMap::new(), &BTreeMap::new());

        assert_eq!(r_a, r_b);
    }
}
