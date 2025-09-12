# TEE Public Key Verification Flow

This document describes the TEE public key verification process.

## Sequence Diagram

```mermaid
sequenceDiagram
    participant Client as HTTP Client
    participant Server as Scheduler Server
    participant TEE as TEE Server
    participant KMS as AWS KMS

    Client->>Server: GET /public_key
    Server->>Server: Decode KMS public key from base64
    Server->>Server: Lock and clone AWS credentials
    
    Server->>Server: Build AwsConfig with:
    Note right of Server: - public_key (decoded bytes)<br/>- kms_key_id<br/>- aws_region<br/>- aws credentials<br/>- encryption algorithm
    
    Server->>TEE: Request::TeePublicKey { aws_config }
    
    Note over TEE: TEE Processing
    TEE->>TEE: Verify public key against KMS
    alt Local Run Mode
        TEE->>TEE: Use local RSA key for verification
    else Production Mode  
        TEE->>KMS: Encrypt test payload with public key
        TEE->>KMS: Decrypt with KMS CLI
        TEE->>TEE: Compare decrypted result
    end
    
    alt Verification Success
        TEE->>TEE: Generate attestation document
        TEE-->>Server: Response::TeePublicKey { public_key, attestation_document }
        Server-->>Client: 200 OK with public key and attestation
    else Verification Failure
        TEE-->>Server: Error response
        Server-->>Client: 500 Internal Server Error
    end
```

## Key Features

- **KMS Integration**: Verifies public key against AWS KMS
- **Attestation**: Returns hardware attestation document in production
- **Local Development**: Supports local RSA key verification
- **Security**: Validates KMS configuration before processing requests
