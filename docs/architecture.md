# Fortiquo Blockchain Architecture

## Overview

Fortiquo is a custom blockchain with:

- **Native ML-DSA-44 Accounts** (not Ethereum)
- **EVM Execution Engine** (not full Ethereum)
- **Proof-of-History Consensus** (inspired by Solana, not copied)
- **Modular Design** (trait-based, swappable components)

This is **not an Ethereum clone**. Ethereum transaction format, account recovery, and secp256k1 are not used.

## Core Design Principles

### 1. Separation of Concerns

- **Account Model**: Native ML-DSA-44, no recovery logic
- **Transaction Format**: Custom format with ML-DSA signatures
- **Consensus**: PoH for ordering + validator finality
- **Smart Contracts**: EVM executes Solidity bytecode only
- **Storage**: Pluggable backend (RocksDB default)

### 2. Type Safety & Modularity

Each crate owns its domain:
- `types`: Domain types and constants
- `crypto`: Signature schemes, address derivation
- `consensus`: PoH recorder, validator scheduling
- `state`: Persistent storage abstraction
- `evm`: EVM execution bridge
- `mempool`: Transaction admission
- `executor`: Block execution orchestration
- `p2p`: Network messages and gossip
- `rpc`: JSON-RPC endpoints
- `wallet-sdk`: Client-side transaction building
- `node`: Binary and service wiring

### 3. No Ethereum Assumptions

| Concept | Ethereum | Fortiquo |
|---------|----------|----------|
| Signature | secp256k1 ECDSA | ML-DSA-44 |
| Account Recovery | Yes (pubkey → address) | No (deterministic address derivation) |
| Transaction Format | RLP + hardcoded | Custom postcard/JSON |
| Contract Execution | EVM (compatible) | EVM (not Ethereum-compatible at chain level) |
| Consensus | PoW/PoS | PoH-style + validator finality |
| Wallet | MetaMask | Fortiquo SDK |

## Transaction Lifecycle

```
┌─────────────┐
│   Wallet    │  (ML-DSA keypair, transaction builder)
└──────┬──────┘
       │ sign(tx) → SignedTransaction
       ▼
┌─────────────────┐
│  JSON-RPC       │  chain_sendRawTransaction
└──────┬──────────┘
       │
       ▼
┌────────────────┐
│  Mempool       │  validate signature, nonce, size
└──────┬─────────┘
       │ gossip to peers
       ▼
┌─────────────────────┐
│  BlockProducer      │  (leader from schedule)
│  (via consensus)    │  select txs, assign to PoH entries
└──────┬──────────────┘
       │
       ▼
┌──────────────────────────┐
│  BlockExecutor           │  for each tx:
│  (Executor service)      │  - validate signature
│                          │  - derive sender address
│                          │  - check nonce, balance
│                          │  - call EVM
│                          │  - generate receipt
└──────┬───────────────────┘
       │ block + receipts
       ▼
┌──────────────────┐
│  BlockVerifier   │  (consensus layer)
│                  │  - verify PoH sequence
│                  │  - verify leader authorization
│                  │  - verify all tx receipts
└──────┬───────────┘
       │ valid block → state update
       ▼
┌──────────────────┐
│  StateStore      │  persist block, receipts, accounts
└──────┬───────────┘
       │
       ▼
┌──────────────────┐
│  Indexer         │  index txs, receipts for RPC queries
└──────────────────┘
```

## Proof-of-History Consensus

PoH provides **verifiable ordering**, not finality.

### PoH Entry

```rust
struct PohEntry {
    previous_hash: Hash,
    current_hash: Hash,
    tick_number: u64,
    tx_hashes: Vec<TxHash>,
    timestamp: Option<u64>,
    leader_id: ValidatorId,
}
```

- `previous_hash` + `current_hash` form a chain
- `tick_number` is sequential
- `tx_hashes` are transactions included in this entry
- `leader_id` identifies the block producer

### PoH Recorder

The recorder maintains the hash chain:

1. Start with `genesis_hash`
2. For each tick:
   - `next_hash = blake3(previous_hash || tick_number || tx_hashes)`
   - Record entry
3. Chain is verifiable: anyone can replay and verify every entry

### PoH Verifier

Validator verifies:

1. Each entry's `previous_hash` matches prior entry's `current_hash`
2. Tick numbers increment by 1
3. Leader ID is authorized for this tick range
4. All transactions included are valid

### Leader Schedule

Deterministic schedule from validator set:

```
slot = poh_tick / TICKS_PER_SLOT
leader_idx = slot % validator_set.len()
leader = validator_set[leader_idx]
```

For MVP: simple round-robin. Future: stake-weighted.

### Validator Finality

**PoH is for ordering. Finality is separate.**

- A finality mechanism (to be extended) decides when a block is final
- MVP: validator authorizes blocks (no Byzantine tolerance)
- Future: BFT-style multi-signature finality

## EVM Execution

**revm handles contract bytecode. Not full Ethereum.**

### Native → EVM

1. `SignedTransaction` arrives
2. Validate ML-DSA signature
3. Derive sender address from public key
4. Map to EVM execution context:
   - sender address
   - target (or create)
   - calldata / bytecode
   - value, gas_limit, gas_price
5. Execute in revm
6. Collect logs, gas_used, output/revert
7. Generate receipt

### State Access

- revm operations read/write contract storage
- Storage persists in `StateStore` (RocksDB)
- No Ethereum-style account abstraction

### Gas Accounting

- `gas_limit` from transaction
- `gas_used` from revm execution
- Refund on non-revert
- Fee: `gas_used * max_fee_per_gas + priority_fee`

## Cryptography

### ML-DSA-44

All user transactions are signed with ML-DSA-44 (FIPS 204):

- Key generation: `keypair(seed) → (public_key, secret_key)`
- Signing: `sign(message, secret_key) → signature`
- Verification: `verify(message, signature, public_key) → bool`

Implementation:
- Trait-based abstraction in `crypto` crate
- Feature-gated: `real-ml-dsa` (production) vs. test implementation
- Address derivation: `address = blake3(public_key)[0..20]`

### No Ethereum Recovery

No public key recovery from signature. Address is derived deterministically from public key only.

## Mempool

Transaction admission pipeline:

1. Parse and deserialize
2. Validate signature (ML-DSA)
3. Check nonce against account state
4. Check balance ≥ gas_limit * max_fee_per_gas
5. Check tx size < limit
6. Detect duplicates
7. Sort by fee (priority_fee descending, then max_fee)
8. Gossip to peers

Eviction policy: (to be extended)
- Age-based timeout
- Replace by fee

## State Storage

RocksDB backend with abstraction:

```rust
pub trait StateStore: Send + Sync {
    fn get_account(&self, addr: &Address) -> Result<Account>;
    fn set_account(&mut self, addr: Address, account: Account) -> Result<()>;
    fn get_contract_code(&self, addr: &Address) -> Result<Vec<u8>>;
    fn set_contract_code(&mut self, addr: Address, code: Vec<u8>) -> Result<()>;
    fn get_storage(&self, addr: &Address, slot: &Hash) -> Result<Hash>;
    fn set_storage(&mut self, addr: Address, slot: Hash, value: Hash) -> Result<()>;
    // ... blocks, receipts, etc.
}
```

Allows swapping RocksDB for in-memory, database, etc.

## P2P Network

libp2p-based with custom message types:

- **NewTransaction**: broadcast pending transactions
- **NewBlock**: broadcast new blocks with PoH entries
- **RequestPohEntries**: sync PoH history
- **ResponsePohEntries**: return PoH entries
- **RequestBlock**: sync missing block
- **ResponseBlock**: return block

Validation hooks ensure only valid messages are gossipped.

## RPC Endpoints

JSON-RPC 2.0 over HTTP/WebSocket:

### Transaction Endpoints

- `chain_sendRawTransaction(tx)` → tx_hash
- `chain_getTransactionByHash(tx_hash)` → tx
- `chain_getTransactionReceipt(tx_hash)` → receipt

### Block Endpoints

- `chain_getBlockByNumber(number)` → block
- `chain_getBlockByHash(hash)` → block

### Account Endpoints

- `chain_getBalance(addr)` → balance
- `chain_getNonce(addr)` → nonce
- `chain_getCode(addr)` → bytecode
- `chain_getStorageAt(addr, slot)` → value

### Contract Endpoints

- `chain_call(tx, block)` → result (read-only execution)
- `chain_estimateGas(tx)` → gas_used

### PoH Endpoints

- `chain_getPohEntry(tick)` → entry
- `chain_getPohRange(start, end)` → entries
- `chain_getValidatorSet(epoch)` → validators
- `chain_getLeaderSchedule(start_slot, count)` → leaders

## Security Model

### Transaction Signing

1. SignedTransaction includes full `public_key`
2. RPC/Mempool verifies `signature` with `public_key`
3. Sender address derived from `public_key` deterministically
4. No signature recovery attack possible

### Replay Protection

1. Chain ID in transaction format
2. Nonce per account (incrementing)
3. Txs with same (chain_id, nonce, sender) are duplicates
4. Only first tx in a block with given (sender, nonce) executes

### PoH Sequence Integrity

1. Every PoH entry links to previous via hash
2. Tampering any entry invalidates all subsequent
3. Validator verifies full PoH sequence before confirming block
4. Network gossip includes full PoH proof chain

### EVM Safety

1. Gas limits prevent infinite loops
2. Contract code immutable after deployment
3. Storage reads/writes isolated by address
4. No EVM-level privilege escalation

## Testing Strategy

Every crate includes:

- **Unit tests**: module-level logic
- **Integration tests**: cross-crate workflows
- **Property tests**: serialization, PoH sequences
- **Benchmarks**: critical paths (optional)

## Future Extensions

1. **Full PoS**: Stake-weighted leader schedule
2. **BFT Finality**: Byzantine fault tolerance
3. **Light Clients**: SPV-style proofs
4. **State Proofs**: Merkle tree verification
5. **Upgradeable Contracts**: Proxy pattern standards
6. **Cross-Chain Bridges**: Attestation scheme

---

This design keeps Fortiquo focused on native ML-DSA accounts and PoH-style consensus while leveraging revm for contract execution. It is not Ethereum and does not attempt to be.
