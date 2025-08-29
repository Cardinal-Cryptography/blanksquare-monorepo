[![LOGO][logo]][blanksquare-homepage]

[![Nightly Testnet E2E tests][nightly-tests-badge]][nightly-tests]
[![Relayer deployment][relayer-deployment-badge]][relayer-deployment]

# BlankSquare Monorepo

BlankSquare is the **composable privacy stack for wallets and DeFi apps**. We deliver plug-and-play privacy infrastructure (Shielder SDK, Shielded Accounts, Relayer Network, Anonymity Revoking mechanisms) that can be integrated directly into EVM-compatible wallets and applications.

Our goal is to make on-chain privacy **usable, scalable, and compliant by default**. With BlankSquare, wallets and dApps can unlock private, auditable interactions for their users without the technical lift, while generating revenue through withdrawal and shielding fees.

**🔒 Audit report** by zkSecurity is available [here](https://reports.zksecurity.xyz/reports/aleph-zero-shielder/).

## Table of Contents

- [Introduction](#introduction)
- [Key Features](#key-features)
- [Architecture](#architecture)
- [Repository Structure](#repository-structure)
- [Getting Started](#getting-started)
  - [Prerequisites](#prerequisites)
  - [Quick Setup](#quick-setup)
  - [Integration Guide](#integration-guide)
- [Development](#development)
- [Testing](#testing)
- [License](#license)

## Introduction

BlankSquare brings privacy to existing blockchain ecosystems rather than building from scratch. By enhancing EVM-compatible chains with zero-knowledge privacy features, we provide immediate privacy solutions for the existing Web3 ecosystem.

### Why BlankSquare?

- **Privacy-as-a-Service**: Integrate ZK-based privacy without deep cryptographic knowledge
- **EVM Compatibility**: Works with existing Ethereum-based applications and infrastructure  
- **Subsecond Proving**: Generate zero-knowledge proofs in 600-800ms on standard hardware
- **DeFi Composability**: Perfectly compatible with existing DeFi protocols and AMMs
- **Developer-Friendly**: Comprehensive SDK and tooling for seamless integration

## Key Features

### 🛡️ Shielded Pools
- **Private Transactions**: Shield any ERC20 tokens and transact privately
- **DeFi Integration**: Use shielded tokens in existing DeFi protocols (AMMs, lending, etc.)
- **Composability**: Full compatibility with existing smart contract ecosystem

### ⚡ High Performance
- **Fast Proofs**: 600-800ms proving times on consumer hardware
- **Scalable**: Supports high-throughput applications
- **EVM Compatible**: Deploy on any EVM-compatible chain

### 🔧 Developer Tools
- **Shielder SDK**: TypeScript/JavaScript SDK for easy integration
- **WASM Support**: Client-side proof generation in browsers
- **Mobile Ready**: Cross-platform mobile SDK available
- **Comprehensive Documentation**: Step-by-step integration guides

### 🔐 Privacy & Compliance  
- **Zero-Knowledge Proofs**: Powered by Halo2 and custom circuits
- **Anonymity Revoking**: Built-in compliance mechanisms when needed
- **Audited**: Security audited by zkSecurity

## Architecture

BlankSquare is built on a modular architecture with the following key components:

### Smart Contracts
- **Shielder Contract**: Core privacy contract for shielded pools
- **Verifier Contracts**: ZK proof verification (deposit, withdraw, new account)
- **Supporting Contracts**: Merkle trees, nullifiers, protocol fees

### Zero-Knowledge Circuits
- **Deposit Circuit**: Proves valid token shielding
- **Withdraw Circuit**: Proves valid private withdrawals  
- **New Account Circuit**: Proves account creation without revealing identity
- **Built with Halo2**: State-of-the-art ZK proof system

### SDK & Tools
- **Shielder SDK**: Main TypeScript SDK for application integration
- **WASM Crypto**: Browser-compatible cryptographic operations
- **Mobile SDK**: Native mobile integration support
- **CLI Tools**: Command-line utilities for development and testing

### Infrastructure
- **Relayer Network**: Gasless transaction submission
- **Fee Estimation**: Dynamic fee calculation for optimal UX

## Repository Structure

This monorepo contains the complete BlankSquare privacy stack:

### `/contracts`
Solidity smart contracts for the Shielder protocol:
- `Shielder.sol` - Main privacy contract with shielded pools
- `*Verifier.sol` - ZK proof verifiers for different operations
- `MerkleTree.sol`, `Nullifiers.sol` - Core privacy primitives

### `/crates` (Rust)
Core Rust crates for cryptography, circuits, and infrastructure:
- `shielder-circuits/` - Halo2 ZK circuits
- `shielder-relayer/` - Relayer service 
- `shielder-cli/` - Command-line interface to Shielder
- `shielder-account/` - Account management and key handling
- `content-encryption/` - Encryption utilities for private data
- `halo2-verifier/` - Solidity verifier generation

### `/ts` (TypeScript)
JavaScript/TypeScript SDK and tooling:
- `shielder-sdk/` - Main SDK for application integration
- `shielder-sdk-crypto/` - Core cryptographic operations
- `shielder-sdk-crypto-wasm/` - Browser-compatible WASM crypto
- `shielder-sdk-crypto-mobile/` - Mobile SDK support

### `/test`, `/tooling-e2e-tests`
Comprehensive test suites including end-to-end integration tests

## Getting Started

### Prerequisites

Before you begin, ensure you have the following:

- **Node.js** (v18 or later) and **npm**
- **Rust** (latest stable) and **Cargo**
- **Foundry** for smart contract development
- **Docker** (optional, for containerized deployment)

### Quick Setup

Clone the repository and install dependencies:

```bash
git clone git@github.com:Cardinal-Cryptography/blanksquare-monorepo.git
cd blanksquare-monorepo
make deps
```

Boot a local development node:

```bash
make anvil
```

Generate and deploy contracts:

```bash
make generate-contracts
make deploy-contracts
```

### Integration Guide

For developers wanting to integrate BlankSquare privacy into their applications, see our [integration documentation](https://docs.blanksquare.io/integration-guides/quickstart).

## Development

### Building from Source

The project uses both Rust and TypeScript components:

```bash
# Build Rust crates
cargo build --release

# Build TypeScript packages
cd ts
pnpm install
pnpm build
```

### Development Environment

For local development

```bash
# Start local blockchain
make anvil

# Deploy contracts in development mode
NETWORK=anvil make deploy-contracts
```

The development setup uses the default Anvil account `0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266` as the deployer.

### Building Docker Images

For deployment or testing in containerized environments:

```bash
# Build relayer image
export BUILD=docker
run tooling-e2e-tests/full_scenario.sh
```

## Testing

### End-to-End Testing

Install dependencies and generate contracts first:

```bash
make deps
make generate-contracts
```

Run the full test scenario:

```bash
run tooling-e2e-tests/full_scenario.sh
```

For maximum compatibility, use Docker mode (slower but more reliable):

```bash
export BUILD=docker
run tooling-e2e-tests/full_scenario.sh
```

### Unit Tests

Run Rust unit tests:

```bash
cargo test
```

Run TypeScript tests:

```bash
cd ts
pnpm test
```

### Integration Tests

The project includes comprehensive integration tests that verify the complete privacy flow across all components. These tests are automatically run in CI and can be executed locally using the scripts in `tooling-e2e-tests/`.

## License

BlankSquare is licensed under multiple licenses:

- **Most components**: [Apache-2.0 License](LICENSE)
- **Circuits** (`crates/shielder-circuits`): GPL-3.0-only-with-Classpath-exception  

See individual component directories for specific license information.

## Security & Audits

BlankSquare has undergone comprehensive security auditing:
- **zkSecurity Audit**: [Full Report](https://reports.zksecurity.xyz/reports/aleph-zero-shielder/)

## Documentation & Resources

- 📚 **Documentation**: [docs.blanksquare.io](https://docs.blanksquare.io/)
- 🏠 **Homepage**: [blanksquare.io](https://blanksquare.io/)
- 🐛 **Issues**: [GitHub Issues](https://github.com/Cardinal-Cryptography/blanksquare-monorepo/issues)

## Contributing

We welcome contributions! Please see our contributing guidelines and feel free to submit issues and pull requests.

## About Cardinal Cryptography

BlankSquare is developed by [Cardinal Cryptography](https://cardinal.co/), building the future of private and scalable blockchain infrastructure.

---

## Development Notes

### Circuits History

Originally, the `shielder-circuits` crate was developed in a [separate repository](https://github.com/Cardinal-Cryptography/zkOS-circuits) and migrated to this monorepo without preserving git history.

[blanksquare-homepage]: https://blanksquare.io/
[logo]: BSQ-header.png
[relayer-deployment]: https://github.com/Cardinal-Cryptography/blanksquare-monorepo/actions/workflows/build-and-deploy-shielder-relayer.yml
[relayer-deployment-badge]: https://github.com/Cardinal-Cryptography/blanksquare-monorepo/actions/workflows/build-and-deploy-shielder-relayer.yml/badge.svg
[nightly-tests]: https://github.com/Cardinal-Cryptography/blanksquare-monorepo/actions/workflows/testnet-nightly-e2e.yml
[nightly-tests-badge]: https://github.com/Cardinal-Cryptography/blanksquare-monorepo/actions/workflows/testnet-nightly-e2e.yml/badge.svg
