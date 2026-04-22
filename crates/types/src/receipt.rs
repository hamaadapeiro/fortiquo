use crate::{Address, BlockHash, Hash, TxHash};
use serde::{Deserialize, Serialize};

/// Execution status of a transaction.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExecutionStatus {
    /// Transaction executed successfully.
    Success,
    /// Transaction reverted.
    Revert,
}

/// A log entry (event) emitted by a smart contract.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LogEntry {
    /// Address that emitted the log.
    pub address: Address,
    /// Topics (indexed parameters).
    pub topics: Vec<Hash>,
    /// Raw data (unindexed parameters).
    pub data: Vec<u8>,
}

impl LogEntry {
    /// Create a new log entry.
    pub fn new(address: Address, topics: Vec<Hash>, data: Vec<u8>) -> Self {
        LogEntry {
            address,
            topics,
            data,
        }
    }
}

/// Receipt for a transaction execution.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Receipt {
    /// Hash of the transaction.
    pub tx_hash: TxHash,
    /// Block number.
    pub block_number: u64,
    /// Block hash.
    pub block_hash: BlockHash,
    /// Transaction index in the block.
    pub tx_index: u32,
    /// Execution status.
    pub status: ExecutionStatus,
    /// Gas used.
    pub gas_used: u64,
    /// Cumulative gas used in the block.
    pub cumulative_gas_used: u64,
    /// Logs emitted.
    pub logs: Vec<LogEntry>,
    /// Contract address (for contract creation).
    pub contract_address: Option<Address>,
    /// Return data or revert reason.
    pub output: Vec<u8>,
}

impl Receipt {
    /// Create a new receipt.
    pub fn new(
        tx_hash: TxHash,
        block_number: u64,
        block_hash: BlockHash,
        tx_index: u32,
        status: ExecutionStatus,
        gas_used: u64,
    ) -> Self {
        Receipt {
            tx_hash,
            block_number,
            block_hash,
            tx_index,
            status,
            gas_used,
            cumulative_gas_used: 0,
            logs: vec![],
            contract_address: None,
            output: vec![],
        }
    }

    /// Set cumulative gas used.
    pub fn set_cumulative_gas(&mut self, cumulative: u64) {
        self.cumulative_gas_used = cumulative;
    }

    /// Add a log.
    pub fn add_log(&mut self, log: LogEntry) {
        self.logs.push(log);
    }

    /// Set contract address (for creations).
    pub fn set_contract_address(&mut self, addr: Address) {
        self.contract_address = Some(addr);
    }

    /// Set output (return data or revert reason).
    pub fn set_output(&mut self, data: Vec<u8>) {
        self.output = data;
    }

    /// Check if execution was successful.
    pub fn is_success(&self) -> bool {
        self.status == ExecutionStatus::Success
    }

    /// Serialize the receipt.
    pub fn serialize(&self) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        Ok(postcard::to_allocvec(self)?)
    }

    /// Deserialize a receipt.
    pub fn deserialize(data: &[u8]) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(postcard::from_bytes(data)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execution_status() {
        assert_eq!(ExecutionStatus::Success, ExecutionStatus::Success);
        assert_ne!(ExecutionStatus::Success, ExecutionStatus::Revert);
    }

    #[test]
    fn test_log_entry_creation() {
        let addr = Address::new([1u8; 20]);
        let topics = vec![Hash::new([2u8; 32])];
        let data = vec![1, 2, 3];

        let log = LogEntry::new(addr, topics.clone(), data.clone());
        assert_eq!(log.address, addr);
        assert_eq!(log.topics, topics);
        assert_eq!(log.data, data);
    }

    #[test]
    fn test_receipt_creation() {
        let tx_hash = TxHash::new(Hash::new([1u8; 32]));
        let block_hash = BlockHash::new(Hash::new([2u8; 32]));

        let receipt = Receipt::new(tx_hash, 1, block_hash, 0, ExecutionStatus::Success, 21_000);

        assert_eq!(receipt.tx_hash, tx_hash);
        assert_eq!(receipt.block_number, 1);
        assert_eq!(receipt.status, ExecutionStatus::Success);
        assert_eq!(receipt.gas_used, 21_000);
        assert!(receipt.is_success());
        assert!(receipt.contract_address.is_none());
    }

    #[test]
    fn test_receipt_modifications() {
        let tx_hash = TxHash::new(Hash::new([1u8; 32]));
        let block_hash = BlockHash::new(Hash::new([2u8; 32]));

        let mut receipt = Receipt::new(tx_hash, 1, block_hash, 0, ExecutionStatus::Success, 50_000);

        receipt.set_cumulative_gas(50_000);
        assert_eq!(receipt.cumulative_gas_used, 50_000);

        let addr = Address::new([5u8; 20]);
        receipt.set_contract_address(addr);
        assert_eq!(receipt.contract_address, Some(addr));

        receipt.set_output(vec![1, 2, 3]);
        assert_eq!(receipt.output, vec![1, 2, 3]);

        let log = LogEntry::new(addr, vec![], vec![42]);
        receipt.add_log(log);
        assert_eq!(receipt.logs.len(), 1);
    }

    #[test]
    fn test_receipt_serialization() {
        let tx_hash = TxHash::new(Hash::new([1u8; 32]));
        let block_hash = BlockHash::new(Hash::new([2u8; 32]));

        let mut receipt = Receipt::new(tx_hash, 5, block_hash, 2, ExecutionStatus::Success, 75_000);
        receipt.set_output(vec![99, 100]);

        let serialized = receipt.serialize().unwrap();
        let deserialized = Receipt::deserialize(&serialized).unwrap();

        assert_eq!(receipt.tx_hash, deserialized.tx_hash);
        assert_eq!(receipt.block_number, deserialized.block_number);
        assert_eq!(receipt.gas_used, deserialized.gas_used);
        assert_eq!(receipt.output, deserialized.output);
    }
}
