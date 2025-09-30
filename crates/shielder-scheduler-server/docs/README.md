# Shielder Scheduler Server Documentation

This directory contains sequence diagrams and documentation for the Shielder Scheduler Server's KMS integration.

## Sequence Diagrams

1. **[TEE Public Key Verification](tee-public-key-verification.md)** - Describes the TEE public key verification process with KMS integration  
2. **[Withdraw Request Processing](withdraw-request-processing.md)** - End-to-end flow from client request to blockchain submission
3. **[KMS Decryption Flow](kms-decryption-flow.md)** - Detailed view of the decryption process within the TEE

## Architecture Overview

The Shielder Scheduler Server integrates with AWS KMS to provide secure cryptographic operations:

- **TEE Security**: Hardware-backed encryption/decryption in AWS Nitro Enclaves
- **KMS Key Management**: Secure DEK decryption using AWS KMS
- **Local Development**: Support for local testing without AWS dependencies

## Key Features

- Thread-safe credential storage and updates
- Hardware attestation for TEE verification
- Secure payload encryption/decryption
