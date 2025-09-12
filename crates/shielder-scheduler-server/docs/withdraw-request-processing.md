# Withdraw Request Processing Flow

This document describes the end-to-end withdraw request processing with KMS integration.

## Sequence Diagram

```mermaid
sequenceDiagram
    participant Client as HTTP Client
    participant Server as Scheduler Server
    participant DB as Database
    participant Processor as Scheduler Processor
    participant TEE as TEE Server
    participant KMS as AWS KMS
    participant Relayer as Relayer Service

    Client->>Server: POST /schedule_withdraw
    Note right of Client: ScheduleWithdrawRequest with:<br/>- encryption_envelope<br/>- last_note_index<br/>- max_relayer_fee<br/>- relay_after<br/>- pocket_money<br/>- token_address

    Server->>Server: Validate request fields
    Server->>Server: Convert relay_after to DateTime
    
    Server->>DB: insert_scheduled_request(encryption_envelope, ...)
    DB-->>Server: request_id
    Server-->>Client: 200 OK { request_id, message }

    Note over Processor: Background Processing
    loop Scheduler Processing
        Processor->>DB: get_pending_requests()
        DB-->>Processor: List of ScheduledRequest
        
        loop For each request
            Processor->>Processor: Lock and clone AWS credentials
            Processor->>Processor: Decode KMS public key from base64
            Processor->>Processor: Build AwsConfig
            
            Processor->>TEE: Request::PrepareRelayCalldata
            Note right of Processor: - aws_config<br/>- encryption_envelope<br/>- merkle_path<br/>- pocket_money
            
            Note over TEE: Decryption & Processing
            alt Local Run Mode
                TEE->>TEE: Decrypt DEK with local RSA key
            else Production Mode
                TEE->>KMS: kmstool_enclave_cli decrypt DEK
                KMS-->>TEE: Decrypted DEK
            end
            
            TEE->>TEE: Decrypt payload with AES-256-GCM
            TEE->>TEE: Parse payload and prepare calldata
            TEE-->>Processor: Relay calldata
            
            Processor->>Relayer: Submit calldata
            Relayer-->>Processor: Submission result
            
            alt Success
                Processor->>DB: Update request status to Completed
            else Failure
                Processor->>DB: Update request status to Failed
            end
        end
    end
```

## Key Components

- **Encryption Envelope**: Contains encrypted payload, DEK, IV, and auth tag
- **KMS Integration**: Secure DEK decryption using AWS KMS
- **Database Persistence**: Stores encrypted data and request state
- **Background Processing**: Asynchronous request processing
- **Relayer Integration**: Submits prepared calldata to blockchain
