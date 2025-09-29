# Shielder Scheduler Server

This service provides the ability to schedule withdrawal requests that will be processed at a specified future time. The service consists of three main components:

1. **HTTP API**: Receives and stores withdrawal requests
2. **Background Scheduler Processor**: Processes requests when their scheduled time arrives
3. **TEE Task Pool**: Manages communication with the Trusted Execution Environment

## Architecture & Security

### AWS Integration & Session Management

The server uses EC2 instance metadata service to dynamically retrieve temporary AWS credentials at startup. This provides enhanced security by:

- Eliminating the need to hardcode long-term AWS credentials in environment variables
- Using temporary session tokens that have a limited lifespan
- Automatically refreshing credentials every 15 minutes (configurable via `AWS_STS_REFRESH_CREDENTIALS_PERIOD_SECONDS`)
- Thread-safe credential updates during runtime without service interruption

The server performs the following AWS-related operations on startup:

1. **EC2 Metadata Authentication**: Calls EC2 instance metadata service to obtain temporary credentials from the assigned IAM role
2. **TEE Verification**: Performs an initial `TeePublicKey` request to verify proper TEE and KMS integration
3. **Credential Storage**: Stores the retrieved credentials in the application state for subsequent use

**Important**: The server will fail to start if AWS authentication or TEE verification fails, ensuring that only properly configured instances can process withdrawal requests.

### KMS Key Verification

The server maintains proper security by verifying the KMS key relationship with the TEE:

- **Startup Verification**: The `Request::TeePublicKey` is called once during server startup to verify the TEE configuration
- **Runtime Verification**: KMS key verification must be performed every time attestation is requested to maintain the correct relationship between the public key in the TEE and the key in KMS
- **Failure Handling**: Any verification failure causes the server to bail, preventing operation with invalid configurations

### Local Development

For local development and testing, use the dedicated local binary that skips AWS KMS integration and uses in-memory storage instead of DynamoDB:

- **Local Binary**: `shielder-scheduler-server-local`
- **No AWS Dependencies**: Uses dummy credentials provider and in-memory storage
- **Simplified Configuration**: Only requires core application parameters

When using the local binary:

- The `shielder-scheduler-tee` must also be run with its local configuration
- AWS parameters (AWS_REGION, KMS_KEY_ID, AWS_IAM_KMS_ROLE) are not required
- No background AWS credential refresh
- No metrics collection (simplified for development)

This allows developers to run the server locally without requiring EC2 instance metadata or proper AWS IAM roles.

## API Endpoints

### 1. Health Check

**GET** `/health`

Simple health check endpoint.

### 2. TEE Public Key

**GET** `/public_key`

Retrieve the TEE public key for encrypting payloads.

### 3. Schedule Withdrawal

**POST** `/schedule_withdraw`

Schedule a withdrawal request to be processed at a future time.

### 4. Get Request Status

**GET** `/status/{last_note_index}`

Retrieve the status of a withdrawal request by its last note index.

#### Parameters

- `last_note_index`: The last note index of the withdrawal request (path parameter)

#### Response

```json
{
  "last_note_index": "12345",
  "status": "pending",
  "created_at": "2024-01-01T00:00:00Z",
  "processed_at": null,
  "relay_after": "2024-01-01T01:00:00Z"
}
```

**Status Values**: `pending`, `processing`, `completed`, `failed`

**Error Response** (404 when not found):
```json
{
  "error": "Request not found",
  "last_note_index": "12345"
}
```

#### Request Body

```json
{
   "encryption_envelope": {
      "encrypted_payload": "<base64-encoded>",
      "encrypted_dek": "<base64-encoded>",
      "iv": "<base64-encoded>",
      "auth_tag": "<base64-encoded>="
   },
  "last_note_index": "12345",
  "pocket_money": "500000000000000000",
  "token_address": "0x1234567890123456789012345678901234567890",
  "relay_after": 1693564800
}
```

- `encryption_envelope`: Object containing encrypted payload and encryption metadata
  - `encrypted_payload`: Base64-encoded encrypted payload containing withdrawal details
  - `encrypted_dek`: Base64-encoded encrypted data encryption key
  - `iv`: Base64-encoded initialization vector
  - `auth_tag`: Base64-encoded authentication tag
- `last_note_index`: Index of the last leaf in the Merkle tree (as string)
- `pocket_money`: Pocket money amount for the withdrawal (as string, in wei)
- `token_address`: Token address for the withdrawal (as hex string)
- `relay_after`: Unix timestamp (seconds) after which the relay is allowed

#### Response

```json
{
  "request_id": "123",
  "message": "Withdraw request scheduled successfully. Request ID: 123"
}
```

## Request Statuses

- **Pending**: Request is waiting to be processed
- **Processing**: Request is being retried or is in progress
- **Completed**: Request has been successfully processed
- **Failed**: Request processing failed and reached max retry attempts count

## Background Processing

The service runs a background scheduler processor that:

1. Checks for pending requests every 5 seconds (configurable via `SCHEDULER_INTERVAL_SECS`)
2. Processes requests whose `relay_after` time has passed
3. Updates request status in the database
4. Handles retries with configurable retry count and delay
5. Communicates with the TEE through a managed task pool
6. Interacts with the relayer service to get fee quotes and submit transactions
7. Validates fees against user-specified maximums before processing

The scheduler processor can handle multiple requests in batches (configurable via `SCHEDULER_BATCH_SIZE`) and provides error handling with automatic retries.

## Configuration

The service can be configured using environment variables or command-line arguments:

### Server Configuration

- `PUBLIC_PORT`: HTTP server port (default: 3000)
- `METRICS_PORT`: Metrics endpoint port (default: 3001)
- `BIND_ADDRESS`: Server bind address (default: 0.0.0.0)
- `MAXIMUM_REQUEST_SIZE`: Maximum request size in bytes (default: 102400)

### TEE Configuration

- `TEE_CID`: TEE context identifier for vsock communication (default: VMADDR_CID_HOST)
- `TEE_PORT`: TEE port for vsock communication (default: 5000)
- `TEE_TASK_POOL_CAPACITY`: Maximum concurrent TEE tasks (default: 100, max: 128)
- `TEE_TASK_POOL_TIMEOUT_SECS`: Task spawn timeout in seconds (default: 5)
- `TEE_COMPUTE_TIMEOUT_SECS`: TEE response timeout in seconds (default: 60)

### Scheduler Processor Configuration

- `SCHEDULER_INTERVAL_SECS`: How often to check for pending requests (default: 5)
- `SCHEDULER_BATCH_SIZE`: Maximum requests to process per batch (default: 10)
- `SCHEDULER_MAX_RETRY_COUNT`: Maximum retry attempts per request (default: 3)
- `SCHEDULER_RETRY_DELAY_SECS`: Delay between retry attempts (default: 60)

### AWS & KMS Configuration

- `KMS_KEY_ID`: AWS KMS key identifier for encryption operations (required for production binary)
- `KMS_PUBLIC_KEY`: Base64-encoded public key for KMS verification (always required)

**Local Development**: When using the `shielder-scheduler-server-local` binary, AWS-related environment variable `KMS_KEY_ID` is not required. The local binary uses in-memory storage and dummy credentials provider, skipping AWS credential retrieval and refresh logic entirely.

**Converting PEM to Base64**: If you have a PEM file, you can convert it to base64:

```bash
# Remove PEM headers/footers and convert to single line base64
cat your-public-key.pem | grep -v "BEGIN\|END" | tr -d '\n'
```

### Blockchain Configuration

- `NODE_RPC_URL`: RPC URL of the Ethereum node to connect to (required)
- `SHIELDER_ADDRESS`: Address of the Shielder contract (required)
- `RELAYER_URL`: URL of the relayer service (required)

### Metrics Configuration

- `METRICS_UPKEEP_TIMEOUT_SECS`: How often to perform metric upkeep (default: 60)
- `METRICS_BUCKET_DURATION_SECS`: Duration of metric histogram buckets (default: 60)

## Architecture

The service is built with clear separation of concerns:

### Components

1. **HTTP API Layer** (`handlers/`):

   - `health.rs`: Health check endpoint
   - `tee_public_key.rs`: TEE public key retrieval
   - `schedule_withdraw.rs`: Withdrawal request scheduling
   - `get_status.rs`: Request status retrieval

2. **Storage Layer** (`storage/`):

   - `dynamo_db.rs`: DynamoDB storage provider for production
   - `in_memory.rs`: In-memory storage provider for local development
   - `mod.rs`: Storage provider trait and common functionality
   - `schema.rs`: Data schemas and models

3. **Credentials Provider** (`credentials_provider/`):

   - `aws_credentials_provider.rs`: Real AWS credentials with automatic refresh
   - `dummy_credentials_provider.rs`: Mock credentials for local development
   - `mod.rs`: Credentials provider trait

4. **Scheduler Processor** (`scheduler_processor.rs`):

   - Background processing of scheduled requests
   - Batch processing with configurable limits
   - Retry logic with backoff
   - Request parameter parsing and validation
   - Fee quotation from relayer services
   - TEE communication for calldata preparation
   - Response processing and relay submission

5. **Relayer Communication** (`relayer_controller.rs`):

   - Communication with external relayer service
   - Fee quotation requests
   - Relay transaction submission

6. **TEE Communication** (`tee_controller.rs`):
   - Managed through a bounded task pool
   - Vsock-based communication with TEE
   - Configurable timeouts and capacity limits

7. **Application Binaries** (`bin/`):
   - `main.rs`: Production binary with full AWS integration
   - `local.rs`: Local development binary with simplified dependencies

### Data Flow

1. Client submits withdrawal request via HTTP API
2. Request is validated and stored in the configured storage backend (DynamoDB for production, in-memory for local development)
3. Background scheduler processor periodically checks for ready requests
4. Ready requests are processed through the following pipeline:
   - Parse and validate request parameters
   - Get fee quote from relayer service
   - Validate fee against user-specified maximum
   - Send request to TEE for calldata preparation
   - Process TEE response and submit to relayer
5. Results are updated in the storage backend with appropriate status

## Example Usage

**Prerequisites**:

- EC2 instance with an IAM role that has KMS permissions
- The EC2 instance must have access to EC2 instance metadata service (IMDSv2)
- The specified KMS key must be accessible and properly configured

**Command Line Usage Examples**:

Production mode (DynamoDB + AWS KMS):

```bash
# Using the production binary
cargo run --bin shielder-scheduler-server --release -- \
  --kms-public-key <base64-key> \
  --kms-key-id <key-id> \
  --node-rpc-url <rpc-url> \
  --shielder-address <contract-addr> \
  --relayer-url <relayer-url>
```

Local development mode (in-memory storage, no AWS integration):

```bash
# Using the local binary
cargo run --bin shielder-scheduler-server-local -- \
  --kms-public-key <base64-key> \
  --node-rpc-url <rpc-url> \
  --shielder-address <contract-addr> \
  --relayer-url <relayer-url>
```

### Generating Test RSA Keys

For local testing, you can generate RSA key pairs using OpenSSL:

```bash
# Generate RSA private key in PKCS#8 format (2048-bit)
openssl genpkey -algorithm RSA -out test_private_key.pem -pkcs8 -pkeyopt rsa_keygen_bits:2048

# Generate corresponding public key
openssl pkey -in test_private_key.pem -pubout -out test_public_key.pem

# Convert public key to DER format and encode as base64 (for KMS_PUBLIC_KEY)
openssl pkey -in test_private_key.pem -pubout -outform DER -out test_public_key.der
base64 -w 0 test_public_key.der > test_public_key_base64.txt

# Use the base64-encoded public key for KMS_PUBLIC_KEY
export KMS_PUBLIC_KEY=$(cat test_public_key_base64.txt)
```

**Security Note**: Test keys should only be used for local development. Never use test keys in production environments.

### Monitoring

The service exposes Prometheus metrics on the `/metrics` endpoint (default port 3001):

```bash
curl http://localhost:3001/metrics
```
