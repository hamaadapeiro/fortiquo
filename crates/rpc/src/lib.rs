//! JSON-RPC 2.0 `chain_*` namespace for Fortiquo (built on [`jsonrpsee`]).
//!
//! Method registration is synchronous; the server runtime is provided by the node binary.

mod context;
mod error;
mod module;

pub use context::{leader_schedule, ChainContext};
pub use error::RpcError;
pub use module::build_rpc_module;
