[![LOGO][logo]][blanksquare-homepage]

[![Nightly Testnet E2E tests][nightly-tests-badge]][nightly-tests]
[![Relayer deployment][relayer-deployment-badge]][relayer-deployment]

# Shielder

Shielder is designed to provide a seamless integration of zero-knowledge (ZK) privacy into Ethereum Virtual Machine (EVM) compatible environments with subsecond proving times.

**Audit report** by zkSecurity is available [here](https://reports.zksecurity.xyz/reports/aleph-zero-shielder/).

## Table of Contents

- [Introduction](#introduction)
- [Features](#features)
- [Architecture](#architecture)
- [Getting Started](#getting-started)
  - [Prerequisites](#prerequisites)
  - [Usage](#usage)
- [License](#license)

## Introduction

Shielder aims at delivering Privacy-as-a-Service for Web3. With Shielder, developers can integrate ZK-based privacy into their applications without deep cryptographic knowledge.

## Features

- **EVM Compatibility**: Easily integrate with existing Ethereum-based applications.
- **Subsecond Proving**: Achieve zero-knowledge proofs in 600-800 ms on standard hardware.
- **High Performance**: Supports up to 250ms block time and processes thousands of transactions per second.
- **Developer-Friendly**: Comprehensive tooling and frameworks to simplify integration.
- **Privacy-Enhanced**: Build and deploy privacy-enhanced applications effortlessly.

# Architecture

Shielder is built utilizing the following components:

- **EVM Layer 2 Rollup**: Leverages Arbitrum Anytrust DAC technology for fast and secure execution.
- **Developer Tooling**: Includes Gelato’s web3 services, account abstraction, functions, VRF, oracles, block explorers, indexers, and multisig support.

## Getting Started

### Prerequisites

Before you begin, ensure you have the following:

- Node.js and npm installed
- Docker (optional, for containerized deployment)
- An Ethereum wallet

### Usage

Clone the repository and install the dependencies:

```bash
git clone git@github.com:Cardinal-Cryptography/blanksquare-monorepo.git
cd blanksquare-monorepo
make deps
```

Boot a local node:

```bash
make anvil
```

Generate additional contract sources:

```bash
make generate-contracts
```

Compile & deploy the smart contract suite:

```bash
make deploy-contracts
```

### Running e2e tests

Install dependencies and generate additional contract sources:

```bash
make deps
make generate-contracts
```

Run:

```bash
run tooling-e2e-tests/full_scenario.sh
```

(or another `.sh` file from that directory). For maximum compatibility `export BUILD=docker` - this will be slower but
will build the relayer image inside docker. Otherwise the binary is built on your machine and then copied into the image
which might not work, depending on the exact configuration of the host.

### Deploying to anvil

The command below will use `0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266` as a deployer:

```bash
NETWORK=anvil make deploy-contracts
```

## License

Most of the Shielder is licensed under the Apache-2.0 License. See the LICENSE file for more details.

Some of the crates are licensed with LICENSE-GPL-3.0-only-with-Classpath-exception:
* `crates/shielder-circuits`

Some of the crates are licensed with MIT:
* `crates/transcript`

## Circuits

Originally, `shielder-circuits` crate was placed in a different [repo](https://github.com/Cardinal-Cryptography/zkOS-circuits) and migrated without 
preserving git history to this repo, so in case one needs to check `git blame`, please visit original repo.

[blanksquare-homepage]: https://blanksquare.io/
[logo]: BSQ-header.png
[relayer-deployment]: https://github.com/Cardinal-Cryptography/blanksquare-monorepo/actions/workflows/build-and-deploy-shielder-relayer.yml
[relayer-deployment-badge]: https://github.com/Cardinal-Cryptography/blanksquare-monorepo/actions/workflows/build-and-deploy-shielder-relayer.yml/badge.svg
[nightly-tests]: https://github.com/Cardinal-Cryptography/blanksquare-monorepo/actions/workflows/testnet-nightly-e2e.yml
[nightly-tests-badge]: https://github.com/Cardinal-Cryptography/blanksquare-monorepo/actions/workflows/testnet-nightly-e2e.yml/badge.svg
