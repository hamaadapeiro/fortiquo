# EVM Execution Model

Fortiquo uses revm for smart contract execution. This document describes how we bridge our custom transaction format to EVM execution.

## Design Principles

1. **EVM is an engine, not the chain**
   - revm executes bytecode only
   - State access goes through our StateStore
   - Transaction semantics are native to Fortiquo

2. **Native transactions, then EVM calls**
   - Validate Fortiquo SignedTransaction
   - Derive sender from ML-DSA public key
   - Convert to EVM ExecutionInput
   - Execute
   - Generate Fortiquo Receipt

3. **State isolation**
   - Contract storage is part of global state
   - No account abstraction beyond ML-DSA
   - No allowance/approval model (use Solidity)

## Transaction to EVM Mapping

### Fortiquo SignedTransaction

```rust
pub struct SignedTransaction {
    pub unsigned_tx: UnsignedTransaction,
    pub public_key: PublicKeyBytes,
    pub signature: SignatureBytes,
    pub algorithm_id: AlgorithmId,
}

pub struct UnsignedTransaction {
    pub chain_id: u64,
    pub nonce: u64,
    pub gas_limit: u64,
    pub max_fee_per_gas: u128,
    pub priority_fee_per_gas: u128,
    pub to: Option<Address>,
    pub value: u128,
    pub data: Vec<u8>,
    pub tx_kind: TransactionKind,
}

pub enum TransactionKind {
    Transfer,
    ContractCall,
    ContractCreate,
}
```

### EVM ExecutionInput

```rust
pub struct EvmExecutionInput {
    pub caller: Address,
    pub target: Option<Address>,
    pub calldata: Vec<u8>,
    pub value: u128,
    pub gas_limit: u64,
    pub gas_price: u128,
}
```

### Mapping Logic

```rust
pub fn native_tx_to_evm(tx: &SignedTransaction) -> Result<EvmExecutionInput> {
    // 1. Validate signature and derive sender
    let sender = crypto::verify_and_derive_address(
        &tx.public_key,
        &tx.signature,
        &tx.algorithm_id,
        tx.serialize_for_signing()?,
    )?;
    
    // 2. Map fields
    Ok(EvmExecutionInput {
        caller: sender,
        target: tx.unsigned_tx.to,
        calldata: tx.unsigned_tx.data.clone(),
        value: tx.unsigned_tx.value,
        gas_limit: tx.unsigned_tx.gas_limit,
        gas_price: tx.unsigned_tx.max_fee_per_gas,
    })
}
```

## Execution Flow

### Contract Create

```
1. UnsignedTransaction has to = None, data = bytecode
2. revm creates new account at address(sender, nonce)
3. revm executes bytecode in CREATE context
4. Contract code stored in state
5. Receipt includes contract_address
```

### Contract Call

```
1. UnsignedTransaction has to = Some(addr), data = calldata
2. revm loads bytecode from StateStore
3. revm executes bytecode in CALL context
4. Storage operations read/write through StateStore
5. Receipt includes logs, output, gas_used
```

### EVM State Access

revm's StatefulAccount trait is implemented by our EvmStateAdapter:

```rust
pub struct EvmStateAdapter<'a> {
    state_store: &'a mut StateStore,
    cache: HashMap<Address, AccountState>,
}

impl EvmStateAdapter {
    pub fn get_balance(&mut self, addr: &Address) -> Result<u128> {
        Ok(self.state_store.get_account(addr)?.balance)
    }
    
    pub fn set_balance(&mut self, addr: Address, balance: u128) -> Result<()> {
        let mut account = self.state_store.get_account(&addr)?;
        account.balance = balance;
        self.state_store.set_account(addr, account)?;
        Ok(())
    }
    
    pub fn get_storage(&mut self, addr: &Address, slot: &Hash) -> Result<Hash> {
        self.state_store.get_storage(addr, slot)
    }
    
    pub fn set_storage(&mut self, addr: Address, slot: Hash, value: Hash) -> Result<()> {
        self.state_store.set_storage(addr, slot, value)
    }
}
```

## Gas Model

### Gas Accounting

- `gas_limit` from transaction (user-specified)
- `gas_used` from revm execution
- Intrinsic gas for transaction overhead (revm computes)
- Refund for storage cleanup (revm computes)

### Fee Calculation

```rust
pub fn calculate_fee(
    gas_used: u64,
    tx: &UnsignedTransaction,
) -> u128 {
    // Base fee + priority fee
    let base_fee = (gas_used as u128) * tx.max_fee_per_gas;
    let priority = (gas_used as u128) * tx.priority_fee_per_gas;
    base_fee + priority
}

pub fn calculate_refund(
    gas_limit: u64,
    gas_used: u64,
    tx: &UnsignedTransaction,
) -> u128 {
    let unused = (gas_limit - gas_used) as u128;
    unused * tx.max_fee_per_gas
}
```

### Balance Validation

Before execution:
```rust
// sender must have balance for gas + value transfer
required_balance = (gas_limit as u128) * tx.max_fee_per_gas + tx.value;
if account.balance < required_balance {
    return Err("Insufficient balance");
}
```

After execution:
```rust
// deduct actual gas + value transfer
fee_paid = calculate_fee(gas_used, tx);
account.balance -= fee_paid + tx.value;
```

## Receipt Generation

```rust
pub struct Receipt {
    pub tx_hash: TxHash,
    pub block_number: u64,
    pub block_hash: BlockHash,
    pub tx_index: u32,
    pub status: ExecutionStatus,  // Success or Revert
    pub gas_used: u64,
    pub cumulative_gas_used: u64,
    pub logs: Vec<LogEntry>,
    pub contract_address: Option<Address>,
    pub output: Vec<u8>,  // return data or revert reason
}

pub enum ExecutionStatus {
    Success,
    Revert,
}
```

Receipts are generated from revm ExecutionResult:

```rust
pub fn evm_result_to_receipt(
    tx_hash: TxHash,
    block_number: u64,
    block_hash: BlockHash,
    tx_index: u32,
    evm_result: ExecutionResult,
) -> Receipt {
    let (status, output) = match evm_result.output {
        Output::Call(data) => (ExecutionStatus::Success, data),
        Output::Create(data, addr) => (ExecutionStatus::Success, data),
        Output::Revert(data) => (ExecutionStatus::Revert, data),
    };
    
    Receipt {
        tx_hash,
        block_number,
        block_hash,
        tx_index,
        status,
        gas_used: evm_result.gas_used,
        cumulative_gas_used: 0,  // set by block executor
        logs: evm_result.logs
            .into_iter()
            .map(|log| LogEntry { ... })
            .collect(),
        contract_address: evm_result.created_address,
        output,
    }
}
```

## Event Logs

Solidity `emit` statements produce EVM logs. We index them:

```rust
pub struct LogEntry {
    pub address: Address,
    pub topics: Vec<Hash>,
    pub data: Vec<u8>,
}
```

Example Solidity:
```solidity
event Transfer(address indexed from, address indexed to, uint256 value);
emit Transfer(msg.sender, recipient, amount);
```

Becomes:
```rust
LogEntry {
    address: token_contract,
    topics: vec![
        keccak256("Transfer(address,address,uint256)"),
        from,
        to,
    ],
    data: value.encode(),
}
```

RPC exposes:
- `chain_getLogs(filter)` for queries
- Indexer stores logs for historical lookup

## Revert Handling

If execution reverts:

```rust
// 1. All state changes except gas deduction are rolled back
// 2. Gas is still consumed (not refunded)
// 3. Receipt status = Revert
// 4. Receipt output = revert reason (REVERT opcode data)
// 5. Transaction is not failed; it's included with error status
```

## Bytecode Validation

No bytecode validation on deployment (Ethereum does via code execution).

Fortiquo accepts any bytecode:
- Valid EVM bytecode: executes
- Invalid bytecode: revert on first execution
- This matches Ethereum behavior

## Testing

Tests verify:
1. Native tx → EVM mapping preserves semantics
2. Contract create assigns correct address
3. Contract call loads correct bytecode
4. Storage operations persist correctly
5. Gas accounting matches transaction spec
6. Receipt generation captures all metadata
7. Logs are indexed correctly
8. Reverts are handled properly

---

EVM is a powerful execution layer. This design keeps it focused on bytecode execution while our native layer handles accounts, transactions, and finality.
