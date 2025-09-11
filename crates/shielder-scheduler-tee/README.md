# Shielder Scheduler TEE

The **Shielder Scheduler TEE** is a Trusted Execution Environment (TEE) component that runs inside AWS Nitro Enclaves to securely handle cryptographic operations for the Shielder privacy protocol. This service provides secure key management and transaction preparation within a hardware-protected environment.

## Overview

This crate implements the TEE-side server that:

- Operates within AWS Nitro Enclaves for hardware-level security guarantees
- Manages cryptographic keys using AWS KMS with attestation
- Prepares relay calldata for private transactions
- Communicates with the host system via vsock (Virtual Socket)
- Provides attestation capabilities to prove execution within a genuine TEE

The TEE server works in conjunction with `shielder-scheduler-server` (the host-side component) to provide a complete scheduled withdrawal service for the Shielder protocol.

## Architecture

```
┌─────────────────────────────────────────┐
│             Host EC2 Instance           │
│  ┌─────────────────────────────────────┐│
│  │      shielder-scheduler-server      ││
│  │           (HTTP API)                ││
│  └─────────────────────────────────────┘│
│                    │                    │
│                  vsock                  │
│  ┌─────────────────────────────────────┐│
│  │       AWS Nitro Enclave (TEE)      ││
│  │  ┌─────────────────────────────────┐││
│  │  │   shielder-scheduler-tee        │││
│  │  │    (Crypto Operations)          │││
│  │  └─────────────────────────────────┘││
│  │              │                     ││
│  │          NSM Driver                ││
│  │              │                     ││
│  │           AWS KMS                  ││
│  └─────────────────────────────────────┘│
└─────────────────────────────────────────┘
```

## Key Features

### 🔒 Hardware Security
- Runs exclusively within AWS Nitro Enclaves
- Hardware-level memory encryption and isolation
- Attestation document generation via NSM (Nitro Security Module)
- Secure communication with AWS KMS

### 🔑 Cryptographic Operations
- **Key Management**: Secure handling of encryption/decryption keys via AWS KMS
- **Data Encryption**: AES-256-GCM encryption for sensitive data
- **RSA Operations**: RSA-OAEP for key exchange and envelope encryption
- **Attestation**: Cryptographic proof of execution environment

### 🌐 Communication
- **vsock Protocol**: Secure communication with host via Virtual Socket
- **Request/Response**: Handles ping, public key retrieval, and relay preparation
- **Async Operations**: Concurrent request handling using Tokio

## Protocol

The TEE server handles the following request types:

### 1. Ping
Simple connectivity test between host and TEE.

**Request:** `Request::Ping`  
**Response:** `Response::Pong`

### 2. TEE Public Key
Retrieves the TEE's public key for secure communication.

**Request:** `Request::TeePublicKey { aws_config }`  
**Response:** `Response::TeePublicKey { public_key, attestation_document }`

### 3. Prepare Relay Calldata
Prepares transaction calldata for relay operations.

**Request:** 
```rust
Request::PrepareRelayCalldata {
    aws_config,
    encryption_envelope,
    relayer_address,
    relayer_fee,
    merkle_path,
}
```

**Response:** `Response::RelayCalldata { calldata }`

## Building

### Prerequisites

- Rust toolchain (see `rust-toolchain.toml`)
- AWS Nitro Enclaves CLI
- Docker (for containerized builds)

### Standard Build

```bash
# Build for production (with attestation)
cargo build --release

# Build for local testing (without attestation)
cargo build --release --features local-run
```

### Docker Build

```bash
# Build the Docker image
docker build -t shielder-scheduler-tee .
```

The Docker build includes:
- Ubuntu 22.04 base image
- AWS KMS Nitro Enclave tools
- Required shared libraries for NSM operations

## Configuration

The TEE server is configured via command-line arguments and environment variables:

| Parameter | Environment | Default | Description |
|-----------|-------------|---------|-------------|
| `--tee-port, -p` | `TEE_PORT` | `42000` | vsock port for TEE communication |
| `--tee-cid` | `TEE_CID` | `VMADDR_CID_ANY` | Context ID for vsock endpoint |
| `--kms-proxy-port` | `KMS_PROXY_PORT` | `8000` | Port for KMS proxy communication |
| `--private-key-base64` | `PRIVATE_KEY_BASE64` | - | Base64-encoded RSA private key for local decryption (replaces AWS KMS, only with `local-run` feature) |

### Example Usage

```bash
# Production mode (inside Nitro Enclave)
./shielder-scheduler-tee --tee-port 42000 --kms-proxy-port 8000

# Local testing mode
./shielder-scheduler-tee --features local-run --private-key-base64 "LS0t..."
```

## Features

### `default`
- Enables AWS Nitro Enclave NSM API integration
- Full attestation capabilities
- Production-ready security

### `local-run`
- Disables NSM driver queries
- Uses provided private key instead of AWS KMS for decryption operations
- Intended for local development and testing only

## Development

### Local Testing

For local development without AWS Nitro Enclaves:

```bash
# Set environment variables
export TEE_PORT=42000
export KMS_PROXY_PORT=8000
export PRIVATE_KEY_BASE64="LS0tLS1CRUdJTi..." # Base64-encoded private key

# Run with test feature
cargo run --features local-run
```

### Linting and Formatting

```bash
# Format code
make format-rust

# Run lints
make lint-rust

# Run tests
make test
```

## Security Considerations

⚠️ **Important Security Notes:**

1. **Production Deployment**: Always use the default feature set in production to ensure proper attestation
2. **Key Management**: Private keys should never be hardcoded or logged
3. **Attestation**: Verify attestation documents before trusting TEE responses
4. **Network Isolation**: TEE should only communicate via vsock with the host
5. **Updates**: Keep AWS Nitro Enclave tools and libraries updated

## Integration

This component integrates with:

- **shielder-scheduler-server**: Host-side HTTP API server
- **shielder-scheduler-common**: Shared protocol definitions
- **AWS KMS**: Key management and attestation
- **AWS Nitro Enclaves**: Hardware security environment

## License

This project is licensed under the same terms as the blanksquare-monorepo parent project.

## Support

For issues related to:
- **TEE Operations**: Check NSM driver and enclave configuration
- **AWS Integration**: Verify KMS permissions and Nitro Enclave setup
- **vsock Communication**: Ensure proper CID/port configuration
- **Local Testing**: Use `local-run` feature for development

See the main repository documentation for deployment guides and troubleshooting.
