# KMS Decryption Flow in TEE

This document describes the KMS-based decryption process within the TEE.

## Sequence Diagram

```mermaid
sequenceDiagram
    participant TEE as TEE Server
    participant KMS_CLI as KMS Tool CLI
    participant KMS as AWS KMS
    participant AES as AES-256-GCM

    Note over TEE: Received EncryptionEnvelope
    TEE->>TEE: Extract encrypted_dek, encrypted_payload, iv, auth_tag

    Note over TEE: DEK Decryption
    alt Production Mode (with shielder-scheduler-server)
        TEE->>KMS_CLI: /usr/local/bin/kmstool_enclave_cli
        Note right of TEE: Args: --region, --key-id,<br/>--ciphertext-blob (base64 DEK)
        KMS_CLI->>KMS: Decrypt API call
        KMS-->>KMS_CLI: Decrypted DEK
        KMS_CLI-->>TEE: PLAINTEXT: <base64_dek>
        TEE->>TEE: Parse and decode DEK from stdout
    else Local Development Mode (with shielder-scheduler-server-local)
        TEE->>TEE: Decrypt DEK with local RSA private key
        Note right of TEE: Using RSA OAEP with SHA-256
    end

    TEE->>TEE: Validate DEK length (must be 32 bytes)
    
    Note over TEE: Payload Decryption
    TEE->>TEE: Concatenate encrypted_payload + auth_tag
    TEE->>AES: Initialize AES-256-GCM with DEK and IV
    AES->>AES: Decrypt and verify authentication
    AES-->>TEE: Plaintext payload
    
    TEE->>TEE: Parse payload as JSON
    TEE->>TEE: Prepare relay calldata with decrypted data

    alt Decryption Success
        TEE-->>TEE: Return prepared calldata
    else Decryption Failure
        TEE-->>TEE: Return KMS error
    end
```

## Security Features

- **Hardware Security**: Decryption occurs within AWS Nitro Enclave
- **Key Isolation**: DEK never exposed outside secure environment  
- **Authenticated Encryption**: AES-256-GCM provides integrity verification
- **Attestation**: Hardware attestation validates TEE authenticity
- **Local Development**: Safe local testing with separate key handling
