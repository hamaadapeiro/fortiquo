use fortiquo_types::Hash;

/// Domain separation prefix for different hash contexts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DomainSeparation {
    /// Transaction hashing.
    Transaction = 1,
    /// Block hashing.
    Block = 2,
    /// PoH hashing.
    ProofOfHistory = 3,
    /// Receipt hashing.
    Receipt = 4,
    /// Account state hashing.
    AccountState = 5,
}

/// Hasher with domain separation for different contexts.
pub struct DomainSeparatedHasher {
    domain: DomainSeparation,
}

impl DomainSeparatedHasher {
    /// Create a new hasher with the given domain.
    pub fn new(domain: DomainSeparation) -> Self {
        DomainSeparatedHasher { domain }
    }

    /// Hash data with domain separation.
    pub fn hash(&self, data: &[u8]) -> Hash {
        let mut hasher = blake3::Hasher::new();
        // Prefix with domain byte for separation
        hasher.update(&[self.domain as u8]);
        hasher.update(data);
        let hash_out = hasher.finalize();
        let mut bytes = [0u8; 32];
        bytes.copy_from_slice(hash_out.as_bytes());
        Hash::new(bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_domain_separation_different_hashes() {
        let data = b"test data";
        let tx_hash = DomainSeparatedHasher::new(DomainSeparation::Transaction).hash(data);
        let block_hash = DomainSeparatedHasher::new(DomainSeparation::Block).hash(data);

        // Different domains should produce different hashes for same data
        assert_ne!(tx_hash, block_hash);
    }

    #[test]
    fn test_same_domain_same_hash() {
        let data = b"test data";
        let hash1 = DomainSeparatedHasher::new(DomainSeparation::Transaction).hash(data);
        let hash2 = DomainSeparatedHasher::new(DomainSeparation::Transaction).hash(data);

        // Same domain should produce same hash (deterministic)
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_all_domains() {
        let domains = vec![
            DomainSeparation::Transaction,
            DomainSeparation::Block,
            DomainSeparation::ProofOfHistory,
            DomainSeparation::Receipt,
            DomainSeparation::AccountState,
        ];

        let data = b"data";
        let mut hashes = Vec::new();

        for domain in domains {
            hashes.push(DomainSeparatedHasher::new(domain).hash(data));
        }

        // All hashes should be different
        for i in 0..hashes.len() {
            for j in (i + 1)..hashes.len() {
                assert_ne!(hashes[i], hashes[j]);
            }
        }
    }
}
