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

For local development and testing, you can use the `--disable-kms` command line flag to skip AWS KMS integration:

- **Command Line Flag**: `--disable-kms`
- **Dummy Credentials**: Uses hardcoded dummy AWS credentials instead of fetching from EC2 metadata
- **No Credential Refresh**: Skips the background AWS credential refresh task when KMS is disabled
- **Validation Changes**: AWS configuration parameters become optional when `--disable-kms` is used

When using `--disable-kms`:
- The `shielder-scheduler-tee` must be run with cargo feature `local-run` and `PRIVATE_KEY_BASE64` environment variable
- AWS parameters (AWS_REGION, KMS_KEY_ID, AWS_IAM_KMS_ROLE) become optional
- The server uses dummy AWS credentials for TEE communication
- Background AWS credential refresh is disabled

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

#### Request Body

```json
{
  "payload": "base64-encoded-encrypted-payload",
  "last_note_index": "12345",
  "max_relayer_fee": "1000000000000000000",
  "pocket_money": "500000000000000000",
  "token_address": "0x1234567890123456789012345678901234567890",
  "relay_after": 1693564800
}
```

- `payload`: Base64-encoded encrypted payload containing withdrawal details
- `last_note_index`: Index of the last leaf in the Merkle tree (as string)
- `max_relayer_fee`: Maximum fee the relayer can charge (as string, in wei)
- `pocket_money`: Pocket money amount for the withdrawal (as string, in wei)
- `token_address`: Token address for the withdrawal (as hex string)
- `relay_after`: Unix timestamp (seconds) after which the relay is allowed

#### Response

```json
{
  "request_id": 123,
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

## Database Schema

The service uses PostgreSQL with the following main table:

```sql
CREATE TABLE scheduled_requests (
    id BIGSERIAL PRIMARY KEY,
    payload BYTEA NOT NULL,
    last_note_index TEXT NOT NULL,
    max_relayer_fee TEXT NOT NULL,
    pocket_money TEXT NOT NULL,
    token_address TEXT NOT NULL,
    relay_after TIMESTAMPTZ NOT NULL,
    status request_status NOT NULL DEFAULT 'pending',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    processed_at TIMESTAMPTZ,
    retry_count INTEGER NOT NULL DEFAULT 0,
    error_message TEXT
);
```

## Configuration

The service can be configured using environment variables or command-line arguments:

### Server Configuration
- `PUBLIC_PORT`: HTTP server port (default: 3000)
- `METRICS_PORT`: Metrics endpoint port (default: 3001)
- `BIND_ADDRESS`: Server bind address (default: 0.0.0.0)
- `MAXIMUM_REQUEST_SIZE`: Maximum request size in bytes (default: 102400)

### Database Configuration
- `DB_HOST`: Database host (default: localhost)
- `DB_PORT`: Database port (default: 5432)
- `DB_NAME`: Database name (default: scheduler-db)
- `DB_USER`: Database user (default: postgres)
- `DB_PASS`: Database password (default: postgres)
- `DB_USE_SSL`: Use SSL for database connection (default: false)

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
- `--disable-kms`: Command line flag to disable KMS and use local private key for decryption (for development only)
- `AWS_REGION`: AWS region for STS and KMS operations (required when KMS is enabled, optional with `--disable-kms`)
- `AWS_IAM_KMS_ROLE`: IAM role name for KMS access (required when KMS is enabled, optional with `--disable-kms`)
- `KMS_KEY_ID`: AWS KMS key identifier for encryption operations (required when KMS is enabled, optional with `--disable-kms`)  
- `KMS_PUBLIC_KEY`: Base64-encoded public key for KMS verification (required)
- `AWS_STS_REFRESH_CREDENTIALS_PERIOD_SECONDS`: How often to refresh AWS STS credentials in seconds (default: 900, range: 900-1800)

**Local Development**: When using the `--disable-kms` flag, AWS-related environment variables become optional. If not provided, dummy AWS credentials are used for local testing. The TEE must be run with cargo feature `local_run` and `PRIVATE_KEY_BASE64` environment variable.

**Converting PEM to Base64**: If you have a PEM file, you can convert it to base64:
```bash
# Remove PEM headers/footers and convert to single line base64
cat your-public-key.pem | grep -v "BEGIN\|END" | tr -d '\n'
```

**Note**: AWS credentials are automatically retrieved using EC2 instance metadata at startup and refreshed periodically. Manual AWS credential configuration is no longer required.

**Token Management**: The metadata token TTL is automatically set to twice the refresh period to ensure adequate overlap during credential rotation.

**Validation**: The refresh period is constrained to 900-1800 seconds (15-30 minutes) to ensure:
- Minimum security through frequent credential rotation (≤30 minutes)
- Reasonable EC2 metadata service usage without excessive calls (≥15 minutes)
- Optimal balance between security and operational efficiency

**STS Refresh Failure Behavior**: If AWS STS credential refresh fails during runtime, the server will shut down gracefully. This ensures that the service does not continue operating with expired credentials, maintaining security compliance. Operators should monitor for service restarts and address any underlying AWS IAM or network connectivity issues that may cause credential refresh failures.

### Blockchain Configuration
- `NODE_RPC_URL`: RPC URL of the Ethereum node to connect to (required)
- `SHIELDER_ADDRESS`: Address of the Shielder contract (required)
- `RELAYER_RPC_URL`: URL of the relayer service (required)


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

2. **Database Layer** (`db/`):
   - PostgreSQL connection management
   - Request storage and retrieval
   - Status tracking and updates

3. **Scheduler Processor** (`scheduler_processor.rs`):
   - Background processing of scheduled requests
   - Batch processing with configurable limits
   - Retry logic with backoff
   - Request parameter parsing and validation
   - Fee quotation from relayer services
   - TEE communication for calldata preparation
   - Response processing and relay submission

4. **Relayer Communication** (`relayer_rpc_controller.rs`):
   - Communication with external relayer service
   - Fee quotation requests
   - Relay transaction submission

5. **TEE Communication**:
   - Managed through a bounded task pool
   - Vsock-based communication with TEE
   - Configurable timeouts and capacity limits

### Data Flow

1. Client submits withdrawal request via HTTP API
2. Request is validated and stored in PostgreSQL database
3. Background scheduler processor periodically checks for ready requests
4. Ready requests are processed through the following pipeline:
   - Parse and validate request parameters
   - Get fee quote from relayer service
   - Validate fee against user-specified maximum
   - Send request to TEE for calldata preparation
   - Process TEE response and submit to relayer
5. Results are updated in the database with appropriate status

## Example Usage

**Prerequisites**:
- EC2 instance with an IAM role that has KMS permissions
- The EC2 instance must have access to EC2 instance metadata service (IMDSv2)
- The IAM role name must match the configured `AWS_IAM_KMS_ROLE`
- The specified KMS key must be accessible and properly configured
- AWS_STS_REFRESH_CREDENTIALS_PERIOD_SECONDS must be between 900 and 1800 seconds

**Validation Examples**:
```bash
# This will fail - refresh period too short
export AWS_STS_REFRESH_CREDENTIALS_PERIOD_SECONDS=600
cargo run  # Error: AWS_STS_REFRESH_CREDENTIALS_PERIOD_SECONDS must be at least 900 seconds

# This will fail - refresh period too long  
export AWS_STS_REFRESH_CREDENTIALS_PERIOD_SECONDS=2000
cargo run  # Error: AWS_STS_REFRESH_CREDENTIALS_PERIOD_SECONDS must be at most 1800 seconds

# This will work - within valid range
export AWS_STS_REFRESH_CREDENTIALS_PERIOD_SECONDS=1200
cargo run  # Success
```

**Command Line Usage Examples**:

Production mode (with KMS):
```bash
cargo run -- --kms-public-key <base64-key> --aws-region us-east-1 --kms-key-id <key-id> --aws-iam-kms-role <iam-role-name> --node-rpc-url <rpc-url> --shielder-address <contract-addr> --relayer-rpc-url <relayer-url>
```

Local development mode (without KMS):
```bash
cargo run -- --disable-kms --kms-public-key <base64-key> --node-rpc-url <rpc-url> --shielder-address <contract-addr> --relayer-rpc-url <relayer-url>
```

### API Testing

Schedule a withdrawal request:

```bash
curl -X POST http://localhost:3000/schedule_withdraw \
  -H "Content-Type: application/json" \
  -d '{
    "payload": "SGVsbG8gV29ybGQ=",
    "last_note_index": "12345",
    "max_relayer_fee": "1000000000000000000",
    "pocket_money": "500000000000000000",
    "token_address": "0x1234567890123456789012345678901234567890",
    "relay_after": 1693564800
  }'
```

Check service health:

```bash
curl http://localhost:3000/health
```

Get TEE public key:

```bash
curl http://localhost:3000/public_key
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