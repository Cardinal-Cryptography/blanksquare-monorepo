# AWS Credentials Management Flow

This document describes the AWS credentials management flow in the Shielder Scheduler Server.

## Sequence Diagram

```mermaid
sequenceDiagram
    participant App as Scheduler Server
    participant IMDS as EC2 Instance Metadata Service
    participant AWS as AWS STS
    participant TEE as TEE Server

    Note over App: Startup Phase
    App->>App: Parse CLI arguments
    App->>App: Validate configuration
    
    alt Production Mode (--disable-kms=false)
        App->>IMDS: PUT /latest/api/token (TTL header)
        IMDS-->>App: Metadata token
        App->>IMDS: GET /latest/meta-data/iam/security-credentials/{role}
        IMDS-->>App: AWS credentials (AccessKeyId, SecretAccessKey, Token)
        App->>App: Store credentials in Arc<Mutex<AwsCredentials>>
    else Local Development Mode (--disable-kms=true)
        App->>App: Use dummy AWS credentials
    end

    App->>TEE: Verify TEE public key
    TEE-->>App: Verification result
    
    Note over App: Background Refresh Task
    loop Every AWS_STS_REFRESH_PERIOD_SECONDS (900-1800s)
        App->>IMDS: PUT /latest/api/token
        IMDS-->>App: New metadata token
        App->>IMDS: GET /latest/meta-data/iam/security-credentials/{role}
        IMDS-->>App: New AWS credentials
        App->>App: Update Arc<Mutex<AwsCredentials>>
        
        alt Credential refresh fails
            App->>App: Log error and signal shutdown
        end
    end
```

## Key Components

- **EC2 Instance Metadata Service (IMDS)**: Provides temporary AWS credentials
- **Credential Rotation**: Automatic refresh every 15-30 minutes
- **Thread-Safe Storage**: Credentials stored in `Arc<Mutex<AwsCredentials>>`
- **Graceful Degradation**: Server shuts down if credential refresh fails
