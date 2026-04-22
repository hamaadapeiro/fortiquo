//! JSON-RPC method registration (`jsonrpsee`).

use crate::context::{leader_schedule, ChainContext};
use crate::RpcError;
use fortiquo_revm::Executor;
use fortiquo_types::{Address, SignedTransaction, TxHash, UnsignedTransaction};
use jsonrpsee::types::error::ErrorObjectOwned;
use jsonrpsee::types::Params;
use jsonrpsee::server::RpcModule;
use std::sync::Arc;

fn decode_hex_0x(s: &str) -> Result<Vec<u8>, RpcError> {
    let s = s.strip_prefix("0x").unwrap_or(s);
    hex::decode(s).map_err(|e| RpcError::InvalidHex(e.to_string()))
}

/// Register all `chain_*` methods on a fresh [`RpcModule`].
pub fn build_rpc_module(ctx: Arc<ChainContext>) -> RpcModule<Arc<ChainContext>> {
    let mut module = RpcModule::new(ctx.clone());

    module
        .register_async_method("chain_sendRawTransaction", |params, ctx, _| async move {
            let hex_tx: String = params.one().map_err(|e| invalid_params(e.to_string()))?;
            let raw = decode_hex_0x(&hex_tx).map_err(rpc_err)?;
            let tx: SignedTransaction =
                postcard::from_bytes(&raw).map_err(|e| rpc_err(RpcError::Decode(format!("{e:?}"))))?;
            if tx.unsigned_tx.chain_id != ctx.chain_id {
                return Err(rpc_err(RpcError::Decode("chain id mismatch".into())));
            }
            Executor::verify_and_derive_sender(&tx).map_err(|e| {
                rpc_err(RpcError::Execution(format!("invalid transaction: {e}")))
            })?;
            let h = tx.hash();
            ctx.tx_cache
                .lock()
                .map_err(|_| internal("tx cache lock"))?
                .insert(h, tx);
            Ok::<String, ErrorObjectOwned>(format!("0x{}", hex::encode(h.as_bytes())))
        })
        .expect("register chain_sendRawTransaction");

    module
        .register_async_method("chain_getTransactionByHash", |params, ctx, _| async move {
            let h_hex: String = params.one().map_err(|e| invalid_params(e.to_string()))?;
            let raw = decode_hex_0x(&h_hex).map_err(rpc_err)?;
            let th = TxHash::new(
                fortiquo_types::Hash::try_from_slice(&raw)
                    .ok_or_else(|| invalid_params("tx hash length".into()))?,
            );
            let cache = ctx
                .tx_cache
                .lock()
                .map_err(|_| internal("tx cache lock"))?;
            let tx = cache
                .get(&th)
                .cloned()
                .ok_or_else(|| rpc_err(RpcError::NotFound(format!("{th}"))))?;
            let v = serde_json::to_value(&tx).map_err(|e| internal(e.to_string()))?;
            Ok::<serde_json::Value, ErrorObjectOwned>(v)
        })
        .expect("register chain_getTransactionByHash");

    module
        .register_async_method("chain_getTransactionReceipt", |params, ctx, _| async move {
            let h_hex: String = params.one().map_err(|e| invalid_params(e.to_string()))?;
            let raw = decode_hex_0x(&h_hex).map_err(rpc_err)?;
            let th = TxHash::new(
                fortiquo_types::Hash::try_from_slice(&raw)
                    .ok_or_else(|| invalid_params("tx hash length".into()))?,
            );
            let st = ctx
                .state
                .lock()
                .map_err(|_| internal("state lock"))?
                .get_receipt(&th)
                .map_err(|e| rpc_err(RpcError::State(e.to_string())))?;
            let v = serde_json::to_value(&st).map_err(|e| internal(e.to_string()))?;
            Ok::<serde_json::Value, ErrorObjectOwned>(v)
        })
        .expect("register chain_getTransactionReceipt");

    module
        .register_async_method("chain_getBlockByNumber", |params, ctx, _| async move {
            let arr: Vec<serde_json::Value> = params
                .parse()
                .map_err(|e| invalid_params(e.to_string()))?;
            let num = arr
                .get(0)
                .and_then(|v| v.as_u64())
                .ok_or_else(|| invalid_params("expected [number]".into()))?;
            let st = ctx
                .state
                .lock()
                .map_err(|_| internal("state lock"))?
                .get_block(num)
                .map_err(|e| rpc_err(RpcError::State(e.to_string())))?;
            let v = serde_json::to_value(&st).map_err(|e| internal(e.to_string()))?;
            Ok::<serde_json::Value, ErrorObjectOwned>(v)
        })
        .expect("register chain_getBlockByNumber");

    module
        .register_async_method("chain_getBalance", |params, ctx, _| async move {
            let addr_hex: String = params.one().map_err(|e| invalid_params(e.to_string()))?;
            let raw = decode_hex_0x(&addr_hex).map_err(rpc_err)?;
            let addr = Address::try_from_slice(&raw)
                .ok_or_else(|| invalid_params("address length".into()))?;
            let acc = ctx
                .state
                .lock()
                .map_err(|_| internal("state lock"))?
                .get_account(&addr)
                .map_err(|e| rpc_err(RpcError::State(e.to_string())))?;
            Ok::<String, ErrorObjectOwned>(acc.balance.to_string())
        })
        .expect("register chain_getBalance");

    module
        .register_async_method("chain_getNonce", |params, ctx, _| async move {
            let addr_hex: String = params.one().map_err(|e| invalid_params(e.to_string()))?;
            let raw = decode_hex_0x(&addr_hex).map_err(rpc_err)?;
            let addr = Address::try_from_slice(&raw)
                .ok_or_else(|| invalid_params("address length".into()))?;
            let acc = ctx
                .state
                .lock()
                .map_err(|_| internal("state lock"))?
                .get_account(&addr)
                .map_err(|e| rpc_err(RpcError::State(e.to_string())))?;
            Ok::<String, ErrorObjectOwned>(acc.nonce.to_string())
        })
        .expect("register chain_getNonce");

    module
        .register_async_method("chain_getCode", |params, ctx, _| async move {
            let addr_hex: String = params.one().map_err(|e| invalid_params(e.to_string()))?;
            let raw = decode_hex_0x(&addr_hex).map_err(rpc_err)?;
            let addr = Address::try_from_slice(&raw)
                .ok_or_else(|| invalid_params("address length".into()))?;
            let code = ctx
                .state
                .lock()
                .map_err(|_| internal("state lock"))?
                .get_contract_code(&addr)
                .map_err(|e| rpc_err(RpcError::State(e.to_string())))?;
            Ok::<String, ErrorObjectOwned>(format!("0x{}", hex::encode(code)))
        })
        .expect("register chain_getCode");

    module
        .register_async_method("chain_estimateGas", |params, _ctx, _| async move {
            let tx_json: serde_json::Value = params.one().map_err(|e| invalid_params(e.to_string()))?;
            let unsigned: UnsignedTransaction =
                serde_json::from_value(tx_json).map_err(|e| invalid_params(e.to_string()))?;
            let gas = if unsigned.data.is_empty()
                && matches!(
                    unsigned.tx_kind,
                    fortiquo_types::TransactionKind::Transfer
                ) {
                21_000u64
            } else {
                unsigned.gas_limit
            };
            Ok::<String, ErrorObjectOwned>(gas.to_string())
        })
        .expect("register chain_estimateGas");

    module
        .register_async_method("chain_getPohEntry", |params, ctx, _| async move {
            let tick: u64 = params.one().map_err(|e| invalid_params(e.to_string()))?;
            let g = ctx
                .poh_cache
                .lock()
                .map_err(|_| internal("poh cache lock"))?;
            let e = g
                .get(&tick)
                .cloned()
                .ok_or_else(|| rpc_err(RpcError::NotFound(format!("tick {tick}"))))?;
            let v = serde_json::to_value(&e).map_err(|e| internal(e.to_string()))?;
            Ok::<serde_json::Value, ErrorObjectOwned>(v)
        })
        .expect("register chain_getPohEntry");

    module
        .register_async_method("chain_getLeaderSchedule", |params, ctx, _| async move {
            let (start_slot, count): (u64, u64) = params
                .parse()
                .map_err(|e| invalid_params(e.to_string()))?;
            let sched = leader_schedule(&ctx);
            let mut ids = Vec::new();
            for i in 0..count {
                ids.push(sched.leader_for_slot(start_slot.saturating_add(i)).id);
            }
            let v = serde_json::to_value(&ids).map_err(|e| internal(e.to_string()))?;
            Ok::<serde_json::Value, ErrorObjectOwned>(v)
        })
        .expect("register chain_getLeaderSchedule");

    module
}

fn invalid_params(msg: String) -> ErrorObjectOwned {
    jsonrpsee::types::error::ErrorObject::owned(-32602, msg, None::<()>)
}

fn internal(msg: String) -> ErrorObjectOwned {
    jsonrpsee::types::error::ErrorObject::owned(-32603, msg, None::<()>)
}

fn rpc_err(e: RpcError) -> ErrorObjectOwned {
    e.into()
}
