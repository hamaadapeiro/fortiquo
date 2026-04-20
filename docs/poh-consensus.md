# Proof-of-History Consensus Design

This document describes Fortiquo's PoH-style consensus mechanism, inspired by Solana's design but implemented independently for our custom blockchain.

## What is Proof-of-History?

PoH is a cryptographic proof that an event occurred at a specific moment in time, relative to other events. It provides:

1. **Verifiable Ordering**: A sequence of transactions can be cryptographically verified to have occurred in that order
2. **Time Encoding**: Tick numbers encode passage of time without relying on wall-clock time
3. **Efficiency**: No BFT consensus needed for ordering; ordering is proven through hash chain

**Critical point**: PoH proves ordering, not finality. Finality requires validator agreement (separate layer).

## Architecture

### 1. PoH Hash Chain

```
PohEntry 0:
  previous_hash = GENESIS_HASH
  current_hash = blake3(GENESIS_HASH || 0 || [])
  tick_number = 0

PohEntry 1:
  previous_hash = Entry(0).current_hash
  current_hash = blake3(Entry(0).current_hash || 1 || [tx_hash_1])
  tick_number = 1
  tx_hashes = [tx_hash_1]

PohEntry 2:
  previous_hash = Entry(1).current_hash
  current_hash = blake3(Entry(1).current_hash || 2 || [])
  tick_number = 2
  tx_hashes = []

...
```

Each entry cryptographically links to the previous one.

### 2. PoH Recorder

The recorder continuously generates entries:

```rust
pub struct PohRecorder {
    current_hash: Hash,
    current_tick: u64,
    leader_id: ValidatorId,
}

impl PohRecorder {
    pub fn new(genesis_hash: Hash, leader: ValidatorId) -> Self { ... }
    
    pub fn tick(&mut self) -> PohEntry {
        // Generate next entry with no transactions
        let entry = PohEntry {
            previous_hash: self.current_hash,
            current_hash: blake3(self.current_hash || self.current_tick || []),
            tick_number: self.current_tick,
            tx_hashes: vec![],
            leader_id: self.leader_id,
        };
        self.current_hash = entry.current_hash;
        self.current_tick += 1;
        entry
    }
    
    pub fn record_transactions(&mut self, tx_hashes: Vec<TxHash>) -> PohEntry {
        // Generate entry with transactions mixed in
        let entry = PohEntry {
            previous_hash: self.current_hash,
            current_hash: blake3(self.current_hash || self.current_tick || tx_hashes),
            tick_number: self.current_tick,
            tx_hashes,
            leader_id: self.leader_id,
        };
        self.current_hash = entry.current_hash;
        self.current_tick += 1;
        entry
    }
}
```

**Key property**: The hash function is deterministic. Anyone with the sequence of entries can verify the hash chain.

### 3. PoH Verifier

Verify a sequence of PoH entries:

```rust
pub struct PohVerifier;

impl PohVerifier {
    pub fn verify_sequence(
        entries: &[PohEntry],
        validators: &ValidatorSet,
        ticks_per_slot: u64,
    ) -> Result<()> {
        for i in 0..entries.len() {
            let entry = &entries[i];
            
            // Check tick is sequential
            if i > 0 && entry.tick_number != entries[i-1].tick_number + 1 {
                return Err("Non-sequential ticks");
            }
            
            // Check hash chain
            if i > 0 {
                let prev = &entries[i-1];
                if entry.previous_hash != prev.current_hash {
                    return Err("Hash chain broken");
                }
            }
            
            // Verify hash computation
            let computed_hash = blake3(
                entry.previous_hash || entry.tick_number || entry.tx_hashes
            );
            if entry.current_hash != computed_hash {
                return Err("Hash mismatch");
            }
            
            // Verify leader is authorized
            let slot = entry.tick_number / ticks_per_slot;
            let leader_idx = slot % validators.len();
            if entry.leader_id != validators[leader_idx].id {
                return Err("Unauthorized leader");
            }
        }
        Ok(())
    }
}
```

**Verification is fast**: Hash chain can be replayed in O(n) time. No multi-signature or voting needed for ordering.

## Slot and Leader Schedule

### Slots and Epochs

- **Tick**: Atomic unit of time (1 per hash computation)
- **Slot**: Group of ticks (e.g., 400 ticks = 1 slot)
- **Epoch**: Group of slots (e.g., 432,000 slots = 1 epoch)

```
Epoch 0:
  Slot 0: Ticks 0-399
    Leader: Validator A
    PohEntries with tick_number 0..399
  
  Slot 1: Ticks 400-799
    Leader: Validator B
    PohEntries with tick_number 400..799
  
  ...
```

### Deterministic Leader Schedule

```rust
pub struct RoundRobinLeaderSchedule {
    validators: Vec<Validator>,
}

impl RoundRobinLeaderSchedule {
    pub fn get_leader(&self, slot: u64) -> &Validator {
        let idx = slot % self.validators.len();
        &self.validators[idx]
    }
}
```

For MVP: simple round-robin. Future: stake-weighted selection.

**Important**: Schedule is deterministic and known in advance. Validators and clients can both compute who should produce each slot.

## Block Production

A leader produces a block by:

1. Starting a new PohRecorder with their ID
2. Ticking continuously (generating PoH entries)
3. When they receive valid mempool transactions, recording them into PoH entries
4. After SLOTS_PER_BLOCK ticks (one slot's worth), collecting all PoH entries + transactions into a block
5. Signing the block
6. Broadcasting to the network

```rust
pub struct BlockProducer {
    recorder: PohRecorder,
    block_buffer: BlockBuffer,
}

impl BlockProducer {
    pub fn produce_block(&mut self, txs: Vec<SignedTransaction>) -> Block {
        let mut poh_entries = vec![];
        
        // Tick and record txs
        for tx in txs {
            let entry = self.recorder.record_transactions(vec![tx.hash()]);
            poh_entries.push(entry);
        }
        
        // Collect PoH entries for this slot
        Block {
            header: BlockHeader {
                parent_hash: ...,
                block_number: ...,
                poh_start_hash: self.recorder.genesis_hash(),
                poh_end_hash: self.recorder.current_hash(),
                poh_start_tick: self.recorder.start_tick(),
                poh_end_tick: self.recorder.current_tick(),
                leader_id: self.recorder.leader_id(),
                timestamp: ...,
            },
            body: BlockBody {
                poh_entries,
                signed_transactions: txs,
            },
        }
    }
}
```

## Block Verification

When a block arrives, the network verifies:

1. **PoH Integrity**: Replay all entries and verify hash chain
2. **Tick Coverage**: Block claims to cover ticks X to Y; verify all entries are included
3. **Leader Authorization**: Check leader is correct for this slot
4. **Transaction Validity**: Verify each transaction's signature, nonce, gas
5. **Execution**: Execute transactions and verify receipts

```rust
pub struct BlockVerifier;

impl BlockVerifier {
    pub fn verify_block(
        block: &Block,
        validators: &ValidatorSet,
        state: &StateStore,
        ticks_per_slot: u64,
    ) -> Result<BlockExecutionResult> {
        // 1. Verify PoH
        PohVerifier::verify_sequence(
            &block.body.poh_entries,
            validators,
            ticks_per_slot,
        )?;
        
        // 2. Verify leader
        let slot = block.header.poh_start_tick / ticks_per_slot;
        let expected_leader = RoundRobinLeaderSchedule::get_leader(&validators, slot);
        if block.header.leader_id != expected_leader.id {
            return Err("Invalid leader");
        }
        
        // 3. Verify transactions
        for tx in &block.body.signed_transactions {
            verify_transaction(tx)?;
        }
        
        // 4. Execute transactions and collect receipts
        let mut execution_result = BlockExecutionResult::new();
        for tx in &block.body.signed_transactions {
            let receipt = execute_transaction(tx, state)?;
            execution_result.receipts.push(receipt);
        }
        
        // 5. Update state
        execution_result.apply_to_state(state)?;
        
        Ok(execution_result)
    }
}
```

## Finality

**PoH proves ordering. It does not prove finality.**

A PoH entry can be cryptographically verified to have occurred before another entry. But it doesn't mean the entry is final (won't be rolled back).

Finality requires validator agreement:

### MVP Finality (Simple)

A block is final when:
1. It has been verified (PoH + transactions valid)
2. The leader has signed it
3. A quorum of validators have acknowledged receipt

Later: Byzantine fault-tolerant finality (e.g., Tendermint-style).

## Consensus Flow

```
┌─────────────────┐
│  PohRecorder    │ (run by current leader)
│  - tick()       │
│  - record_txs() │
└────────┬────────┘
         │ PoH entries
         ▼
┌──────────────────┐
│  BlockProducer   │ (when slot ends)
│  - collect txs   │
│  - emit block    │
└────────┬─────────┘
         │ block with PoH entries
         ▼
┌──────────────────┐
│  Network Gossip  │ (broadcast to all peers)
└────────┬─────────┘
         │
         ▼
┌──────────────────────────┐
│  BlockVerifier (peers)   │
│  - verify PoH sequence   │
│  - verify leader         │
│  - execute transactions  │
│  - update state          │
└────────┬─────────────────┘
         │ valid block
         ▼
┌────────────────────┐
│  FinalizationLayer │ (later: BFT, voting)
│  - collect votes   │
│  - declare final   │
└────────────────────┘
```

## Implementation Notes

### Why Not Just Use Wall-Clock Time?

- Validators have skewed clocks
- Network delays vary
- Attacking time source is trivial
- PoH is cryptographically verifiable, time is not

### Why not Full Solana?

We implement our own PoH to:
- Understand the design deeply
- Customize for our EVM execution model
- Avoid Solana-specific assumptions (SVM, account model)
- Learn from their architecture without copying code

### Scalability

PoH throughput is bounded by:
- Hash computation speed (fast)
- Network gossip latency
- Transaction execution latency (EVM is slow)

Trade-offs:
- Increase `ticks_per_slot` → more throughput, higher latency
- Decrease → lower latency, lower throughput

## Testing

Tests verify:
1. Hash chain generation (deterministic)
2. Hash chain verification (detect tampering)
3. Leader schedule determinism
4. Block production and verification
5. Invalid PoH detection

---

This PoH design provides cryptographically verifiable ordering without requiring Byzantine consensus for the ordering layer. Finality is handled separately by validator agreement.
