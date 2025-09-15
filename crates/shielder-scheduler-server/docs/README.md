# Shielder Scheduler Server Documentation

This directory contains sequence diagrams and documentation for the Shielder Scheduler Server's KMS integration.

## Sequence Diagrams

1. **[AWS Credentials Management Flow](aws-credentials-flow.md)** - Shows how the server obtains and refreshes AWS credentials from EC2 Instance Metadata Service
2. **[TEE Public Key Verification](tee-public-key-verification.md)** - Describes the TEE public key verification process with KMS integration  
3. **[Withdraw Request Processing](withdraw-request-processing.md)** - End-to-end flow from client request to blockchain submission
4. **[KMS Decryption Flow](kms-decryption-flow.md)** - Detailed view of the decryption process within the TEE

## Architecture Overview

The Shielder Scheduler Server integrates with AWS KMS to provide secure cryptographic operations:

- **AWS Integration**: Automatic credential management via EC2 Instance Metadata Service
- **TEE Security**: Hardware-backed encryption/decryption in AWS Nitro Enclaves
- **KMS Key Management**: Secure DEK decryption using AWS KMS
- **Local Development**: Support for local testing without AWS dependencies

## Key Features

- Automatic AWS credential rotation (15-30 minute intervals)
- Thread-safe credential storage and updates
- Hardware attestation for TEE verification
- Secure payload encryption/decryption
- Graceful error handling and server shutdown on credential failures
