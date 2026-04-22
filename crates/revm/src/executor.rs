use crate::error::ExecutionError;
use crate::state::{StateChanges, StateManager};
use crate::EvmConfig;
use fortiquo_crypto::address_deriver::Blake3AddressDeriver;
use fortiquo_crypto::ml_dsa::MlDsa44Scheme;
use fortiquo_crypto::schemes::PublicKeyScheme;
use fortiquo_crypto::AddressDeriver;
use fortiquo_types::{
    Address, AlgorithmId, ExecutionStatus, LogEntry, Receipt, SignedTransaction, TxHash,
    UnsignedTransaction,
};
use std::collections::HashMap;

/// Result of executing a transaction.
#[derive(Clone, Debug)]
pub struct ExecutionResult {
    /// Whether execution was successful
    pub success: bool,
    /// Gas used
    pub gas_used: u64,
    /// Return data or revert reason
    pub output: Vec<u8>,
    /// Logs emitted
    pub logs: Vec<LogEntry>,
    /// Contract address (for creations)
    pub contract_address: Option<Address>,
    /// State changes
    pub state_changes: StateChanges,
}

impl ExecutionResult {
    /// Create a successful result.
    pub fn success(gas_used: u64, output: Vec<u8>) -> Self {
        ExecutionResult {
            success: true,
            gas_used,
            output,
            logs: vec![],
            contract_address: None,
            state_changes: StateChanges {
                modified_accounts: HashMap::new(),
                deleted_accounts: vec![],
                storage_changes: HashMap::new(),
                gas_used,
            },
        }
    }

    /// Create a reverted result.
    pub fn reverted(gas_used: u64, reason: Vec<u8>) -> Self {
        ExecutionResult {
            success: false,
            gas_used,
            output: reason,
            logs: vec![],
            contract_address: None,
            state_changes: StateChanges {
                modified_accounts: HashMap::new(),
                deleted_accounts: vec![],
                storage_changes: HashMap::new(),
                gas_used,
            },
        }
    }
}

/// EVM executor for processing transactions.
pub struct Executor {
    config: EvmConfig,
    state: StateManager,
}

impl Executor {
    /// Create a new executor with default config.
    pub fn new() -> Self {
        Executor {
            config: EvmConfig::default(),
            state: StateManager::new(),
        }
    }

    /// Create a new executor with custom config.
    pub fn with_config(config: EvmConfig) -> Self {
        Executor {
            config,
            state: StateManager::new(),
        }
    }

    /// Execute an unsigned transaction (not yet validated).
    pub fn execute_unsigned(&mut self, tx: &UnsignedTransaction) -> Result<ExecutionResult, ExecutionError> {
        // Validate gas
        self.config.gas_config.validate_gas(0, tx.gas_limit)?;

        // Check chain ID
        if tx.chain_id != self.config.chain_id {
            return Err(ExecutionError::InvalidTransaction(
                "Invalid chain ID".to_string(),
            ));
        }

        // For now, simulate successful execution (MVP)
        // Full EVM integration would happen here with revm
        let gas_used = 21_000; // Base transfer cost

        Ok(ExecutionResult::success(gas_used, vec![]))
    }

    /// Derive the sender address from the ML-DSA-44 public key (never trust `to` for sender).
    pub fn derive_sender_address(tx: &SignedTransaction) -> Result<Address, ExecutionError> {
        let deriver = Blake3AddressDeriver;
        deriver
            .derive_address(&tx.public_key)
            .map_err(|e| ExecutionError::InvalidTransaction(format!("address derive: {e}")))
    }

    /// Verify ML-DSA-44 signature and derive the sender address.
    pub fn verify_and_derive_sender(tx: &SignedTransaction) -> Result<Address, ExecutionError> {
        if tx.algorithm_id != AlgorithmId::MlDsa44 {
            return Err(ExecutionError::InvalidTransaction(
                "unsupported algorithm".into(),
            ));
        }
        let scheme = MlDsa44Scheme;
        let msg = tx
            .signature_bytes()
            .map_err(|e| ExecutionError::SerializationError(e.to_string()))?;
        let ok = scheme
            .verify(&msg, &tx.signature, &tx.public_key)
            .map_err(|e| ExecutionError::InvalidTransaction(format!("verify: {e}")))?;
        if !ok {
            return Err(ExecutionError::InvalidTransaction(
                "invalid signature".into(),
            ));
        }
        Self::derive_sender_address(tx)
    }

    /// Execute a signed transaction (full validation and execution).
    pub fn execute_signed(
        &mut self,
        tx: &SignedTransaction,
    ) -> Result<ExecutionResult, ExecutionError> {
        let sender = Self::verify_and_derive_sender(tx)?;
        self.execute_signed_for_sender(tx, sender)
    }

    /// Execute a signed transaction for an already-verified sender address.
    pub fn execute_signed_for_sender(
        &mut self,
        tx: &SignedTransaction,
        sender: Address,
    ) -> Result<ExecutionResult, ExecutionError> {
        if tx.unsigned_tx.chain_id != self.config.chain_id {
            return Err(ExecutionError::InvalidTransaction(
                "Invalid chain ID".to_string(),
            ));
        }

        let mut sender_account = self.state.get_account(&sender);

        if sender_account.nonce > tx.unsigned_tx.nonce {
            return Err(ExecutionError::InvalidNonce);
        }

        let total_cost = tx.unsigned_tx.value
            + (tx.unsigned_tx.gas_limit as u128 * tx.unsigned_tx.max_fee_per_gas);
        if sender_account.balance < total_cost {
            return Err(ExecutionError::InsufficientBalance);
        }

        let result = self.execute_unsigned(&tx.unsigned_tx)?;

        sender_account.increment_nonce();
        sender_account
            .subtract_balance(result.gas_used as u128 * tx.unsigned_tx.max_fee_per_gas)
            .map_err(|_| ExecutionError::InsufficientBalance)?;

        let mut changes = result.state_changes;
        changes.modified_accounts.insert(sender, sender_account);

        self.state.commit_changes(changes.clone())?;

        Ok(ExecutionResult {
            state_changes: changes,
            ..result
        })
    }

    /// Create a receipt for executed transaction.
    pub fn create_receipt(
        &self,
        tx_hash: TxHash,
        block_number: u64,
        tx_index: u32,
        result: &ExecutionResult,
    ) -> Receipt {
        let status = if result.success {
            ExecutionStatus::Success
        } else {
            ExecutionStatus::Revert
        };

        let mut receipt = Receipt::new(
            tx_hash,
            block_number,
            fortiquo_types::BlockHash::new(fortiquo_types::Hash::zero()),
            tx_index,
            status,
            result.gas_used,
        );

        receipt.set_output(result.output.clone());

        if let Some(addr) = result.contract_address {
            receipt.set_contract_address(addr);
        }

        for log in &result.logs {
            receipt.add_log(log.clone());
        }

        receipt
    }

    /// Get the current state manager.
    pub fn state_mut(&mut self) -> &mut StateManager {
        &mut self.state
    }

    /// Get the current state manager (read-only).
    pub fn state(&self) -> &StateManager {
        &self.state
    }

    /// Get the configuration.
    pub fn config(&self) -> &EvmConfig {
        &self.config
    }
}

impl Default for Executor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fortiquo_types::{AlgorithmId, PublicKeyBytes, SignatureBytes, TransactionKind};

    #[test]
    fn test_executor_creation() {
        let executor = Executor::new();
        assert_eq!(executor.config.chain_id, 1);
    }

    #[test]
    fn test_execute_unsigned_transaction() {
        let mut executor = Executor::new();
        let tx = UnsignedTransaction {
            chain_id: 1,
            nonce: 0,
            gas_limit: 21_000,
            max_fee_per_gas: 1_000_000_000,
            priority_fee_per_gas: 0,
            to: Some(Address::new([1u8; 20])),
            value: 100,
            data: vec![],
            tx_kind: TransactionKind::Transfer,
            memo: None,
        };

        let result = executor.execute_unsigned(&tx).unwrap();
        assert!(result.success);
        assert_eq!(result.gas_used, 21_000);
    }

    #[test]
    fn test_execute_unsigned_wrong_chain() {
        let mut executor = Executor::new();
        let tx = UnsignedTransaction {
            chain_id: 2, // wrong chain
            nonce: 0,
            gas_limit: 21_000,
            max_fee_per_gas: 1_000_000_000,
            priority_fee_per_gas: 0,
            to: Some(Address::new([1u8; 20])),
            value: 100,
            data: vec![],
            tx_kind: TransactionKind::Transfer,
            memo: None,
        };

        let result = executor.execute_unsigned(&tx);
        assert!(result.is_err());
    }

    #[test]
    fn test_receipt_creation() {
        let executor = Executor::new();
        let tx_hash = fortiquo_types::TxHash::new(fortiquo_types::Hash::new([1u8; 32]));
        let result = ExecutionResult::success(21_000, vec![]);

        let receipt = executor.create_receipt(tx_hash, 1, 0, &result);
        assert_eq!(receipt.status, ExecutionStatus::Success);
        assert_eq!(receipt.gas_used, 21_000);
    }

    #[test]
    fn test_receipt_reverted() {
        let executor = Executor::new();
        let tx_hash = fortiquo_types::TxHash::new(fortiquo_types::Hash::new([1u8; 32]));
        let result = ExecutionResult::reverted(21_000, b"error".to_vec());

        let receipt = executor.create_receipt(tx_hash, 1, 0, &result);
        assert_eq!(receipt.status, ExecutionStatus::Revert);
        assert!(!receipt.is_success());
    }
}
