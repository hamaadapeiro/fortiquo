use crate::error::ExecutionError;
use fortiquo_types::{Account, Address, Hash};
use std::collections::HashMap;

/// State changes from a transaction execution.
#[derive(Clone, Debug)]
pub struct StateChanges {
    /// Accounts that were modified
    pub modified_accounts: HashMap<Address, Account>,
    /// Accounts that were deleted
    pub deleted_accounts: Vec<Address>,
    /// Storage changes
    pub storage_changes: HashMap<Address, HashMap<Hash, Vec<u8>>>,
    /// Total gas used
    pub gas_used: u64,
}

/// In-memory state manager for EVM execution.
#[derive(Clone, Debug)]
pub struct StateManager {
    /// Current state of accounts
    accounts: HashMap<Address, Account>,
    /// Account storage (account -> storage key -> value)
    storage: HashMap<Address, HashMap<Hash, Vec<u8>>>,
}

impl StateManager {
    /// Create a new state manager.
    pub fn new() -> Self {
        StateManager {
            accounts: HashMap::new(),
            storage: HashMap::new(),
        }
    }

    /// Get account state, or create empty account if not found.
    pub fn get_account(&self, address: &Address) -> Account {
        self.accounts
            .get(address)
            .cloned()
            .unwrap_or_else(|| Account::new(0))
    }

    /// Set account state.
    pub fn set_account(&mut self, address: Address, account: Account) {
        self.accounts.insert(address, account);
    }

    /// Check if account exists (non-empty).
    pub fn account_exists(&self, address: &Address) -> bool {
        self.accounts
            .get(address)
            .map(|a| !a.is_empty())
            .unwrap_or(false)
    }

    /// Delete an account.
    pub fn delete_account(&mut self, address: Address) {
        self.accounts.remove(&address);
        self.storage.remove(&address);
    }

    /// Get storage value.
    pub fn get_storage(&self, address: &Address, key: &Hash) -> Vec<u8> {
        self.storage
            .get(address)
            .and_then(|storage| storage.get(key))
            .cloned()
            .unwrap_or_default()
    }

    /// Set storage value.
    pub fn set_storage(&mut self, address: Address, key: Hash, value: Vec<u8>) {
        self.storage
            .entry(address)
            .or_insert_with(HashMap::new)
            .insert(key, value);
    }

    /// Commit state changes (apply transaction execution results).
    pub fn commit_changes(&mut self, changes: StateChanges) -> Result<(), ExecutionError> {
        // Apply account modifications
        for (address, account) in changes.modified_accounts {
            self.set_account(address, account);
        }

        // Delete accounts
        for address in changes.deleted_accounts {
            self.delete_account(address);
        }

        // Apply storage changes
        for (address, storage) in changes.storage_changes {
            for (key, value) in storage {
                self.set_storage(address, key, value);
            }
        }

        Ok(())
    }

    /// Get all accounts (for testing/debugging).
    pub fn get_all_accounts(&self) -> &HashMap<Address, Account> {
        &self.accounts
    }

    /// Clear all state (reset).
    pub fn clear(&mut self) {
        self.accounts.clear();
        self.storage.clear();
    }

    /// Create a checkpoint of current state (for reverting).
    pub fn checkpoint(&self) -> StateSnapshot {
        StateSnapshot {
            accounts: self.accounts.clone(),
            storage: self.storage.clone(),
        }
    }

    /// Restore from a checkpoint.
    pub fn restore(&mut self, snapshot: StateSnapshot) {
        self.accounts = snapshot.accounts;
        self.storage = snapshot.storage;
    }
}

impl Default for StateManager {
    fn default() -> Self {
        Self::new()
    }
}

/// A snapshot of the entire state at a point in time.
#[derive(Clone, Debug)]
pub struct StateSnapshot {
    accounts: HashMap<Address, Account>,
    storage: HashMap<Address, HashMap<Hash, Vec<u8>>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_state_manager_new() {
        let state = StateManager::new();
        assert!(state.get_all_accounts().is_empty());
    }

    #[test]
    fn test_set_get_account() {
        let mut state = StateManager::new();
        let addr = Address::new([1u8; 20]);
        let account = Account::new(1_000_000);

        state.set_account(addr, account.clone());
        let retrieved = state.get_account(&addr);

        assert_eq!(retrieved.balance, 1_000_000);
    }

    #[test]
    fn test_account_exists() {
        let mut state = StateManager::new();
        let addr = Address::new([2u8; 20]);

        assert!(!state.account_exists(&addr));
        state.set_account(addr, Account::new(100));
        assert!(state.account_exists(&addr));
    }

    #[test]
    fn test_storage_operations() {
        let mut state = StateManager::new();
        let addr = Address::new([3u8; 20]);
        let key = Hash::new([1u8; 32]);
        let value = vec![1, 2, 3, 4, 5];

        state.set_storage(addr, key, value.clone());
        let retrieved = state.get_storage(&addr, &key);

        assert_eq!(retrieved, value);
    }

    #[test]
    fn test_checkpoint_restore() {
        let mut state = StateManager::new();
        let addr = Address::new([4u8; 20]);
        state.set_account(addr, Account::new(500));

        let checkpoint = state.checkpoint();

        // Modify state
        state.clear();
        assert!(state.get_all_accounts().is_empty());

        // Restore
        state.restore(checkpoint);
        assert_eq!(state.get_account(&addr).balance, 500);
    }

    #[test]
    fn test_delete_account() {
        let mut state = StateManager::new();
        let addr = Address::new([5u8; 20]);
        state.set_account(addr, Account::new(1000));

        assert!(state.account_exists(&addr));
        state.delete_account(addr);
        assert!(!state.account_exists(&addr));
    }
}
