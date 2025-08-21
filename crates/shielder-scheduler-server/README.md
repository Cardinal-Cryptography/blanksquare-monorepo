# Shielder Scheduler Server

This service provides the ability to schedule withdrawal requests that will be processed at a specified future time. The service consists of three main components:

1. **HTTP API**: Receives and stores withdrawal requests
2. **Background Scheduler Processor**: Processes requests when their scheduled time arrives
3. **TEE Task Pool**: Manages communication with the Trusted Execution Environment

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
  "relay_after": 1693564800
}
```

- `payload`: Base64-encoded encrypted payload containing withdrawal details
- `last_note_index`: Index of the last leaf in the Merkle tree (as string)
- `max_relayer_fee`: Maximum fee the relayer can charge (as string, in wei)
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
- **Processing**: Request is currently being processed
- **Completed**: Request has been successfully processed
- **Failed**: Request processing failed
- **Cancelled**: Request has been cancelled

## Background Processing

The service runs a background scheduler processor that:

1. Checks for pending requests every 5 seconds (configurable via `SCHEDULER_INTERVAL_SECS`)
2. Processes requests whose `relay_after` time has passed
3. Updates request status in the database
4. Handles retries with configurable retry count and delay
5. Communicates with the TEE through a managed task pool

The scheduler processor can handle multiple requests in batches (configurable via `SCHEDULER_BATCH_SIZE`) and provides error handling with automatic retries.

## Database Schema

The service uses PostgreSQL with the following main table:

```sql
CREATE TABLE scheduled_requests (
    id BIGSERIAL PRIMARY KEY,
    payload BYTEA NOT NULL,
    last_note_index TEXT NOT NULL,
    max_relayer_fee TEXT NOT NULL,
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
- `DB_PORT`: Database port (default: 5440)
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
   - Retry logic with exponential backoff

4. **TEE Communication**:
   - Managed through a bounded task pool
   - Vsock-based communication with TEE
   - Configurable timeouts and capacity limits

### Data Flow

1. Client submits withdrawal request via HTTP API
2. Request is validated and stored in PostgreSQL database
3. Background scheduler processor periodically checks for ready requests
4. Ready requests are processed through the TEE task pool
5. Results are updated in the database with appropriate status

## Example Usage

### Running the Service

```bash
# With default configuration
cargo run

# With custom configuration
cargo run -- --public-port 8080 --db-host mydb.example.com --scheduler-interval-secs 10

# Using environment variables
export DB_HOST=mydb.example.com
export PUBLIC_PORT=8080
export SCHEDULER_INTERVAL_SECS=10
cargo run
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

### Monitoring

The service exposes Prometheus metrics on the `/metrics` endpoint (default port 3001):

```bash
curl http://localhost:3001/metrics
```
