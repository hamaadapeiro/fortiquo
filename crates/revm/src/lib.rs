//! EVM execution engine for Fortiquo using revm.
//!
//! This crate provides:
//! - Transaction execution with state management
//! - Gas accounting and metering
//! - Contract deployment and interaction
//! - Receipt generation and logs
//!
//! Internally uses revm for efficient bytecode execution.

pub mod executor;
pub mod gas;
pub mod state;
pub mod error;

pub use executor::Executor;
pub use gas::GasConfig;
pub use state::StateManager;
pub use error::ExecutionError;

/// EVM execution configuration
#[derive(Clone, Debug)]
pub struct EvmConfig {
    /// Gas configuration
    pub gas_config: GasConfig,
    /// Chain ID for replay protection
    pub chain_id: u64,
}

impl Default for EvmConfig {
    fn default() -> Self {
        EvmConfig {
            gas_config: GasConfig::default(),
            chain_id: 1,
        }
    }
}
