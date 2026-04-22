//! JSON-RPC error mapping.

use jsonrpsee::types::error::{ErrorObject, ErrorObjectOwned};
use thiserror::Error;

/// RPC-layer failures (mapped to JSON-RPC 2.0 error objects).
#[derive(Debug, Error)]
pub enum RpcError {
    #[error("invalid hex: {0}")]
    InvalidHex(String),

    #[error("decode error: {0}")]
    Decode(String),

    #[error("not found: {0}")]
    NotFound(String),

    #[error("state error: {0}")]
    State(String),

    #[error("execution error: {0}")]
    Execution(String),
}

const INVALID_PARAMS: i32 = -32602;
const INTERNAL: i32 = -32603;

impl From<RpcError> for ErrorObjectOwned {
    fn from(value: RpcError) -> Self {
        let (code, msg) = match &value {
            RpcError::InvalidHex(_) | RpcError::Decode(_) => (INVALID_PARAMS, value.to_string()),
            RpcError::NotFound(_) => (INTERNAL, value.to_string()),
            RpcError::State(_) | RpcError::Execution(_) => (INTERNAL, value.to_string()),
        };
        ErrorObject::owned(code, msg, None::<()>)
    }
}
