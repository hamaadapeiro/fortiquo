# Security Model

This document outlines the security assumptions, threat model, and mitigations for Fortiquo.

## Threat Model

### In Scope

1. **Transaction-Level Attacks**
   - Signature forgery
   - Transaction replay
   - Nonce reuse
   - Double spend

2. **Ordering Attacks**
   - PoH chain tampering
   - Out-of-order transaction execution
   - Unauthorized leader production

3. **Smart Contract Attacks**
   - Reentrancy (mitigated by EVM)
   - Arithmetic overflow (mitigated by Solidity 0.8+)
   - Access control (user responsibility)

4. **Network Attacks**
   - Peer spam
   - Sybil attacks

### Out of Scope (MVP)

- Byzantine validator consensus (future: BFT finality)
- Long-range attacks (future: validator slashing)
- MEV mitigation (future: encrypted transactions, ordering fairness)
- Zero-knowledge proof systems

## Cryptographic Assumptions

### ML-DSA-44 (FIPS 204)

**Assumption**: ML-DSA-44 is a secure post-quantum signature scheme.

- Key generation produces unique, random keypairs
- Signatures are unforgeable under chosen-message attack
- Verification is deterministic

**Implementation**: 
- Feature-gated with `real-ml-dsa` flag
- Production: Use NIST-approved ML-DSA crate (e.g., ml-dsa from FIPS 204)
- Testing: Simple deterministic stub (marked non-production)

**No Recovery**:
- Unlike secp256k1, ML-DSA does not support key recovery
- Address is derived deterministically from public key only
- Public key must be transmitted with transaction
- This is intentional; it's safer than recovery

### BLAKE3 Hashing

**Assumption**: BLAKE3 is a cryptographically secure hash function.

Used for:
- Transaction hashing
- Block hashing
- Address derivation
- PoH hash chain

**Note**: BLAKE3 is not NIST-standardized but is widely used and peer-reviewed.

## Transaction Security

### Signature Validation

**Flow**:
```
1. Receive SignedTransaction with public_key, signature
2. Verify signature: verify(tx.unsigned_tx.serialize(), signature, public_key)
3. Derive sender: address = blake3(public_key)[0..20]
4. Use derived address for nonce, balance checks
5. Never trust public_key from untrusted source; verify against signature
```

**Protection**: Forgery of signature is computationally infeasible (ML-DSA security).

### Replay Protection

**Mechanism**:
- Every transaction includes `chain_id`
- Transactions are unique by `(sender, nonce, chain_id)`
- Mempool deduplicates by `(sender, nonce)`
- State maintains nonce; only increments on successful execution

**Protection**:
- Same transaction cannot be replayed on different chain (chain_id mismatch)
- Same transaction cannot execute twice on same chain (nonce advances)
- Nonce gaps prevent replaying old transactions

### Nonce Ordering

**Rules**:
1. Nonce must be exactly `account.nonce` when executed
2. Nonce increments by 1 on successful execution
3. Failed transactions (revert) still advance nonce
4. Mempool holds transactions for future nonces

**Protection**: Out-of-order execution is impossible.

## PoH Security

### Hash Chain Integrity

**Mechanism**:
```
Each PohEntry has:
  previous_hash (link to prior entry)
  current_hash (computed from previous_hash || tick_number || tx_hashes)
```

**Protection**:
- Tampering any entry invalidates all subsequent
- Verifier can detect tampering in O(n) time
- No BFT voting needed; integrity is cryptographic

### Leader Authorization

**Mechanism**:
```
Deterministic leader schedule from validator set:
  leader = validators[slot_number % validators.len()]
```

**Protection**:
- Only authorized leader can produce for their slot
- Unauthorized block is rejected
- Peers can verify schedule independently

**Limitation (MVP)**:
- No rotating validator set (static for epoch)
- Future: PoS with on-chain validator management

## EVM Safety

### Gas Limits

- Every transaction specifies `gas_limit`
- revm enforces per-opcode gas costs
- Infinite loops are prevented by gas exhaustion

**Protection**: Denial-of-service loops in contracts are bounded.

### Storage Isolation

- Contract A cannot read/write contract B's storage
- Storage is namespaced by (contract_address, storage_slot)

**Protection**: Contracts cannot interfere with each other.

### Code Immutability

- Contract bytecode is immutable after deployment
- (Future: proxy patterns for upgradeability, if needed)

**Protection**: Deployed contracts cannot be replaced.

## State Security

### Database Integrity

- RocksDB uses checksums and write-ahead logging
- State is committed to disk after block validation
- Crash recovery via WAL

**Limitation**: RocksDB is not Byzantine-tolerant (single operator).

### Account Nonces and Balances

- Nonces are checked before execution
- Balances are verified (sufficient for gas + value)
- State updates are atomic per transaction

**Protection**: Double-spend and nonce reuse are prevented.

## Network Security

### Peer Validation

- Incoming transactions are validated before mempool
- Invalid transactions are dropped (no gossip)
- Peer reputation is not tracked (future improvement)

**Limitation**: Sybil attacks and peer spam are possible.

### Message Serialization

- Messages use postcard (binary) for efficiency
- Deserialization validates structure
- Oversized messages are dropped

**Protection**: Malformed messages don't crash node.

## Deployment Safety

### Code Review

- All core modules are expected to be reviewed before production
- Consensus and EVM execution are high-risk
- Tests must be comprehensive

### Monitoring

- Logging via tracing; structured for analysis
- RPC exposes chain state for external verification
- (Future: metrics for anomaly detection)

### Gradual Rollout

- MVP on private testnet first
- Validator set rotation during testing
- Upgrade path for breaking changes

## Known Limitations

### MVP Does Not Include

1. **Finality**
   - PoH provides ordering, not finality
   - Blocks can be reorganized (future: BFT layer)

2. **Validator Rotation**
   - Validator set is static per epoch
   - Cannot add/remove validators mid-epoch

3. **Slashing**
   - No economic penalty for misbehavior
   - Validators are trusted (future: PoS)

4. **MEV Protection**
   - Transactions are ordered transparently
   - Front-running and sandwich attacks are possible
   - (Future: threshold encryption, threshold decryption)

5. **Upgrade Safety**
   - No governance mechanism for chain upgrades
   - Hard fork required for protocol changes

### Future Improvements

1. BFT finality layer (Tendermint, Hotstuff-style)
2. Proof-of-Stake validator set management
3. Validator slashing for Byzantine behavior
4. MEV mitigation (PBS, threshold encryption)
5. Sharding for higher throughput
6. State pruning and light client support

## Security Checklist for Production

Before mainnet:

- [ ] ML-DSA implementation verified by cryptography expert
- [ ] All tests pass including property-based tests
- [ ] Fuzzing on transaction parsing and serialization
- [ ] Audit of consensus and EVM adapter logic
- [ ] Testnet with multiple independent validators
- [ ] Testnet with adversarial transactions (reverts, max gas)
- [ ] Network partition and recovery testing
- [ ] Disk failure and recovery testing
- [ ] Large transaction volume stress test
- [ ] Public code review and disclosure period
- [ ] Monitoring and alerting in place
- [ ] Runbook for security incidents
- [ ] Insurance or circuit breaker for funds

## Responsible Disclosure

If you find a security issue:

1. **Do not** open a public issue
2. Email security@fortiquo.example.com with:
   - Description of vulnerability
   - Steps to reproduce
   - Potential impact
   - Suggested fix (optional)
3. Give us 90 days to respond and publish fix
4. We will credit you in release notes

---

Fortiquo is built on cryptographic security (PoH + ML-DSA) and clean architecture. This provides a solid foundation for a blockchain. As with all software, security is an ongoing process.
