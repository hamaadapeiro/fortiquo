# Fortiquo: A Custom Blockchain in Rust

A production-grade blockchain with native ML-DSA-44 accounts, EVM smart contract execution, and Proof-of-History-inspired consensus.

## Key Features

- **Native ML-DSA-44 Accounts** – Post-quantum cryptography ready, not Ethereum
- **EVM Smart Contracts** – Solidity bytecode execution via revm
- **PoH-Style Consensus** – Verifiable ordering inspired by Solana architecture
- **Custom Transaction Format** – Native ML-DSA signatures, custom gas model
- **Modular Rust Workspace** – Clean separation, independently testable crates
- **Production Architecture** – Config-driven, logging, error handling, full tests

## Architecture

```
crates/
├── types/         # Core blockchain types
├── crypto/        # ML-DSA key management and address derivation
├── consensus/     # PoH recorder, verifier, leader schedule
├── state/         # RocksDB state store
├── evm/           # revm executor adapter
├── mempool/       # Transaction admission and priority
├── executor/      # Block execution pipeline
├── p2p/           # libp2p networking
├── rpc/           # JSON-RPC service
├── wallet-sdk/    # Wallet and transaction builder
└── node/          # Node binary and service orchestration
```

## Building

```bash
cargo build --release
```

## Testing

```bash
cargo test --workspace
```

## Running a Local Chain

(Details in Phase 11+)

## Documentation

- [Architecture Overview](docs/architecture.md)
- [PoH Consensus Design](docs/poh-consensus.md)
- [EVM Execution](docs/evm-execution.md)
- [Security Model](docs/security.md)

## Status

MVP scaffold under active development. Phases 0–14 in progress.

## License

MIT or Apache 2.0
