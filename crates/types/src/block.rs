use crate::{BlockHash, Hash, SignedTransaction, ValidatorId};
use serde::{Deserialize, Serialize};

/// Block number (height in the chain).
pub type BlockNumber = u64;

/// Block header containing metadata about a block.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BlockHeader {
    /// Block number (height).
    pub number: BlockNumber,
    /// Hash of the parent block.
    pub parent_hash: BlockHash,
    /// Root of the state merkle tree.
    pub state_root: Hash,
    /// Root of transactions merkle tree.
    pub tx_root: Hash,
    /// Root of receipts merkle tree.
    pub receipts_root: Hash,
    /// PoH hash at the start of this block.
    pub poh_start_hash: Hash,
    /// PoH hash at the end of this block.
    pub poh_end_hash: Hash,
    /// PoH tick number at the start.
    pub poh_start_tick: u64,
    /// PoH tick number at the end.
    pub poh_end_tick: u64,
    /// Leader (validator) who produced this block.
    pub leader_id: ValidatorId,
    /// Timestamp of the block.
    pub timestamp: u64,
    /// Accumulated gas used in all transactions.
    pub gas_used: u64,
    /// Gas limit for this block.
    pub gas_limit: u64,
}

impl BlockHeader {
    /// Compute the hash of this block header.
    pub fn hash(&self) -> BlockHash {
        let bytes = postcard::to_allocvec(self).expect("serialization failed");
        let hash = Hash::compute(&bytes);
        BlockHash::new(hash)
    }
}

/// A PoH entry containing hash chain data.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PohEntry {
    /// Previous hash in the PoH chain.
    pub previous_hash: Hash,
    /// Current hash in the PoH chain.
    pub current_hash: Hash,
    /// Tick number.
    pub tick_number: u64,
    /// Hashes of transactions included in this entry.
    pub tx_hashes: Vec<crate::TxHash>,
    /// Leader ID.
    pub leader_id: ValidatorId,
}

/// Block body containing transactions and PoH entries.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BlockBody {
    /// PoH entries for this block.
    pub poh_entries: Vec<PohEntry>,
    /// Signed transactions in this block.
    pub signed_transactions: Vec<SignedTransaction>,
}

/// A complete block (header + body).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Block {
    /// Block header.
    pub header: BlockHeader,
    /// Block body.
    pub body: BlockBody,
}

impl Block {
    /// Create a new block.
    pub fn new(header: BlockHeader, body: BlockBody) -> Self {
        Block { header, body }
    }

    /// Get the block hash.
    pub fn hash(&self) -> BlockHash {
        self.header.hash()
    }

    /// Get the number of transactions in this block.
    pub fn tx_count(&self) -> usize {
        self.body.signed_transactions.len()
    }

    /// Get the number of PoH entries.
    pub fn poh_entry_count(&self) -> usize {
        self.body.poh_entries.len()
    }

    /// Serialize the block.
    pub fn serialize(&self) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        Ok(postcard::to_allocvec(self)?)
    }

    /// Deserialize a block.
    pub fn deserialize(data: &[u8]) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(postcard::from_bytes(data)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_block_header_hash() {
        let header = BlockHeader {
            number: 1,
            parent_hash: BlockHash::new(Hash::new([1u8; 32])),
            state_root: Hash::new([2u8; 32]),
            tx_root: Hash::new([3u8; 32]),
            receipts_root: Hash::new([4u8; 32]),
            poh_start_hash: Hash::new([5u8; 32]),
            poh_end_hash: Hash::new([6u8; 32]),
            poh_start_tick: 0,
            poh_end_tick: 399,
            leader_id: ValidatorId::new([7u8; 32]),
            timestamp: 1_000_000,
            gas_used: 50_000,
            gas_limit: 100_000,
        };

        let hash1 = header.hash();
        let hash2 = header.hash();
        assert_eq!(hash1, hash2); // deterministic

        // Different header gets different hash
        let mut header2 = header.clone();
        header2.number = 2;
        let hash3 = header2.hash();
        assert_ne!(hash1, hash3);
    }

    #[test]
    fn test_block_new() {
        let header = BlockHeader {
            number: 0,
            parent_hash: BlockHash::new(Hash::zero()),
            state_root: Hash::zero(),
            tx_root: Hash::zero(),
            receipts_root: Hash::zero(),
            poh_start_hash: Hash::zero(),
            poh_end_hash: Hash::new([1u8; 32]),
            poh_start_tick: 0,
            poh_end_tick: 399,
            leader_id: ValidatorId::new([0u8; 32]),
            timestamp: 0,
            gas_used: 0,
            gas_limit: 100_000,
        };

        let body = BlockBody {
            poh_entries: vec![],
            signed_transactions: vec![],
        };

        let block = Block::new(header.clone(), body);
        assert_eq!(block.header.number, 0);
        assert_eq!(block.tx_count(), 0);
        assert_eq!(block.poh_entry_count(), 0);
    }

    #[test]
    fn test_poh_entry() {
        let entry = PohEntry {
            previous_hash: Hash::new([1u8; 32]),
            current_hash: Hash::new([2u8; 32]),
            tick_number: 42,
            tx_hashes: vec![],
            leader_id: ValidatorId::new([3u8; 32]),
        };

        assert_eq!(entry.tick_number, 42);
        assert_eq!(entry.tx_hashes.len(), 0);
    }

    #[test]
    fn test_block_serialization() {
        let header = BlockHeader {
            number: 1,
            parent_hash: BlockHash::new(Hash::new([1u8; 32])),
            state_root: Hash::new([2u8; 32]),
            tx_root: Hash::new([3u8; 32]),
            receipts_root: Hash::new([4u8; 32]),
            poh_start_hash: Hash::new([5u8; 32]),
            poh_end_hash: Hash::new([6u8; 32]),
            poh_start_tick: 0,
            poh_end_tick: 399,
            leader_id: ValidatorId::new([7u8; 32]),
            timestamp: 1_000_000,
            gas_used: 50_000,
            gas_limit: 100_000,
        };

        let body = BlockBody {
            poh_entries: vec![],
            signed_transactions: vec![],
        };

        let block = Block::new(header, body);
        let serialized = block.serialize().unwrap();
        let deserialized = Block::deserialize(&serialized).unwrap();

        assert_eq!(block.header.number, deserialized.header.number);
        assert_eq!(block.header.gas_used, deserialized.header.gas_used);
    }
}
