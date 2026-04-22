use crate::Hash;
use serde::{Deserialize, Serialize};

/// Account nonce (transaction counter).
pub type Nonce = u64;

/// Account state stored in the state database.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Account {
    /// Account nonce (increments with each transaction).
    pub nonce: Nonce,
    /// Account balance in wei.
    pub balance: u128,
    /// Hash of the contract code (zero if not a contract).
    pub code_hash: Hash,
    /// Root of the contract storage trie.
    pub storage_root: Hash,
}

impl Account {
    /// Create a new account with initial balance.
    pub fn new(balance: u128) -> Self {
        Account {
            nonce: 0,
            balance,
            code_hash: Hash::zero(),
            storage_root: Hash::zero(),
        }
    }

    /// Create a new empty account.
    pub fn empty() -> Self {
        Account {
            nonce: 0,
            balance: 0,
            code_hash: Hash::zero(),
            storage_root: Hash::zero(),
        }
    }

    /// Check if the account is empty (no balance, no code, nonce 0).
    pub fn is_empty(&self) -> bool {
        self.nonce == 0 && self.balance == 0 && self.code_hash == Hash::zero()
    }

    /// Increment nonce.
    pub fn increment_nonce(&mut self) {
        self.nonce = self.nonce.saturating_add(1);
    }

    /// Add balance.
    pub fn add_balance(&mut self, amount: u128) {
        self.balance = self.balance.saturating_add(amount);
    }

    /// Subtract balance (panic if insufficient).
    pub fn subtract_balance(&mut self, amount: u128) -> Result<(), String> {
        if self.balance < amount {
            return Err("Insufficient balance".to_string());
        }
        self.balance -= amount;
        Ok(())
    }

    /// Set code hash.
    pub fn set_code_hash(&mut self, hash: Hash) {
        self.code_hash = hash;
    }

    /// Set storage root.
    pub fn set_storage_root(&mut self, root: Hash) {
        self.storage_root = root;
    }

    /// Serialize the account.
    pub fn serialize(&self) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        Ok(postcard::to_allocvec(self)?)
    }

    /// Deserialize an account.
    pub fn deserialize(data: &[u8]) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(postcard::from_bytes(data)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_account_creation() {
        let account = Account::new(1_000_000_000_000_000_000); // 1 ether
        assert_eq!(account.nonce, 0);
        assert_eq!(account.balance, 1_000_000_000_000_000_000);
        assert_eq!(account.code_hash, Hash::zero());
    }

    #[test]
    fn test_account_empty() {
        let empty = Account::empty();
        assert!(empty.is_empty());

        let with_balance = Account::new(1);
        assert!(!with_balance.is_empty());

        let mut with_nonce = Account::empty();
        with_nonce.nonce = 1;
        assert!(!with_nonce.is_empty());
    }

    #[test]
    fn test_account_nonce_increment() {
        let mut account = Account::empty();
        assert_eq!(account.nonce, 0);

        account.increment_nonce();
        assert_eq!(account.nonce, 1);

        account.increment_nonce();
        assert_eq!(account.nonce, 2);
    }

    #[test]
    fn test_account_balance_add() {
        let mut account = Account::new(100);
        account.add_balance(50);
        assert_eq!(account.balance, 150);

        // Test saturation
        account.balance = u128::MAX - 10;
        account.add_balance(20);
        assert_eq!(account.balance, u128::MAX); // saturates
    }

    #[test]
    fn test_account_balance_subtract() {
        let mut account = Account::new(100);
        account.subtract_balance(30).unwrap();
        assert_eq!(account.balance, 70);

        // Insufficient balance
        let result = account.subtract_balance(100);
        assert!(result.is_err());
        assert_eq!(account.balance, 70); // unchanged
    }

    #[test]
    fn test_account_code_hash() {
        let mut account = Account::empty();
        let code_hash = Hash::new([42u8; 32]);
        account.set_code_hash(code_hash);
        assert_eq!(account.code_hash, code_hash);
    }

    #[test]
    fn test_account_serialization() {
        let mut account = Account::new(1_000_000);
        account.increment_nonce();
        account.set_code_hash(Hash::new([99u8; 32]));

        let serialized = account.serialize().unwrap();
        let deserialized = Account::deserialize(&serialized).unwrap();

        assert_eq!(account.nonce, deserialized.nonce);
        assert_eq!(account.balance, deserialized.balance);
        assert_eq!(account.code_hash, deserialized.code_hash);
    }
}
