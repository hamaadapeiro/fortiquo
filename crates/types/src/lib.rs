//! Core blockchain types for Fortiquo.
//!
//! This crate defines the fundamental types used throughout the blockchain:
//! - Addresses, hashes, and transaction identifiers
//! - Transactions (signed and unsigned)
//! - Blocks and block headers
//! - Receipts and logs
//! - Validators and account state
//!
//! All types are designed for serialization, hashing, and strong typing.

pub mod account;
pub mod address;
pub mod block;
pub mod constants;
pub mod error;
pub mod hash;
pub mod receipt;
pub mod transaction;
pub mod validator;

pub use account::{Account, Nonce};
pub use address::Address;
pub use block::{Block, BlockBody, BlockHeader, BlockNumber};
pub use error::TypeError;
pub use hash::{BlockHash, Hash, TxHash};
pub use receipt::{ExecutionStatus, LogEntry, Receipt};
pub use transaction::{
    AlgorithmId, PublicKeyBytes, SignatureBytes, SignedTransaction, TransactionKind,
    UnsignedTransaction,
};
pub use validator::{Validator, ValidatorId};
