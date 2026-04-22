/// Transaction size limit in bytes.
pub const MAX_TRANSACTION_SIZE: usize = 128 * 1024; // 128 KB

/// Block size limit in bytes.
pub const MAX_BLOCK_SIZE: usize = 10 * 1024 * 1024; // 10 MB

/// Gas limit per block.
pub const MAX_BLOCK_GAS: u64 = 30_000_000;

/// Gas limit per transaction.
pub const MAX_TRANSACTION_GAS: u64 = 30_000_000;

/// Base gas for a transfer transaction.
pub const BASE_GAS_TRANSFER: u64 = 21_000;

/// Base gas for a contract call.
pub const BASE_GAS_CALL: u64 = 21_000;

/// Ticks per slot in PoH.
pub const TICKS_PER_SLOT: u64 = 400;

/// Slots per epoch.
pub const SLOTS_PER_EPOCH: u64 = 432_000;

/// ML-DSA-44 public key size in bytes.
pub const ML_DSA_44_PUBLIC_KEY_SIZE: usize = 1184;

/// ML-DSA-44 signature size in bytes.
pub const ML_DSA_44_SIGNATURE_SIZE: usize = 4668;
