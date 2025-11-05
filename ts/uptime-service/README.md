# Uptime Monitoring Service

A lightweight Node.js/Bun service that monitors health endpoints and exposes metrics in Prometheus format for Grafana dashboards and alerting.

## Prerequisites

- [Bun](https://bun.sh/) installed on your system
- Services with health check endpoints to monitor

## Installation

1. Clone or download this repository
2. Install dependencies:

   ```bash
   bun install
   ```

3. Create a `.env` file based on `.env.example`:

   ```bash
   cp .env.example .env
   ```

4. Configure your endpoints in the `.env` file

## Configuration

All configuration is done via environment variables:

| Variable         | Description                         | Default | Required |
| ---------------- | ----------------------------------- | ------- | -------- |
| `PORT`           | Port for the metrics server         | `9090`  | No       |
| `PROBE_INTERVAL` | Interval between health checks (ms) | `30000` | No       |
| `TIMEOUT`        | HTTP request timeout (ms)           | `5000`  | No       |
| `ENDPOINTS`      | JSON array of endpoints to monitor  | -       | Yes      |

### Endpoint Configuration

The `ENDPOINTS` variable should contain a JSON array with the following structure:

```json
[
  {
    "name": "api-service",
    "url": "http://api.example.com/health",
    "method": "GET",
    "expectedStatus": 200
  },
  {
    "name": "database",
    "url": "http://localhost:5432/health"
  }
]
```

**Endpoint fields:**

- `name` (required): Unique identifier for the service
- `url` (required): Full URL of the health endpoint
- `method` (optional): HTTP method, defaults to `GET`
- `expectedStatus` (optional): Expected HTTP status code, defaults to `200`

### Example Configuration

```env
PORT=9090
PROBE_INTERVAL=30000
TIMEOUT=5000
ENDPOINTS='[
  {"name":"frontend","url":"http://localhost:3000/health"},
  {"name":"backend-api","url":"http://localhost:8080/health","expectedStatus":200},
  {"name":"redis","url":"http://localhost:6379/health"}
]'
```

## Running the Service

### Development Mode

```bash
bun run dev
```

This runs the service with auto-reload on file changes.

### Production Mode

```bash
bun start
```

Or run directly:

```bash
bun run src/index.js
```

## Exposed Endpoints

The service exposes the following HTTP endpoints:

- **`/metrics`** - Prometheus metrics endpoint (for scraping)
- **`/health`** - Health check for the service itself
- **`/`** - Service information and available endpoints

## Prometheus Metrics

The service exposes the following metrics:

### `service_up`

**Type:** Gauge  
**Description:** Service availability status (1 = up, 0 = down)  
**Labels:** `service_name`, `endpoint`

### `service_response_time_seconds`

**Type:** Histogram  
**Description:** Service response time in seconds  
**Labels:** `service_name`, `endpoint`  
**Buckets:** 0.001, 0.01, 0.1, 0.5, 1, 2, 5, 10

### `service_last_probe_timestamp`

**Type:** Gauge  
**Description:** Unix timestamp of the last probe attempt  
**Labels:** `service_name`, `endpoint`

## Grafana Dashboard

### Example Queries

**Current Uptime Status:**

```promql
service_up
```

**Uptime Percentage (last 24h):**

```promql
avg_over_time(service_up[24h]) * 100
```

**Average Response Time:**

```promql
rate(service_response_time_seconds_sum[5m]) / rate(service_response_time_seconds_count[5m])
```

### Alert Rules

**Service Down Alert:**

```yaml
groups:
  - name: uptime_alerts
    rules:
      - alert: ServiceDown
        expr: service_up == 0
        for: 2m
        labels:
          severity: critical
        annotations:
          summary: "Service {{ $labels.service_name }} is down"
          description: "{{ $labels.service_name }} has been down for more than 2 minutes"
```

**High Response Time Alert:**

```yaml
- alert: HighResponseTime
  expr: rate(service_response_time_seconds_sum[5m]) / rate(service_response_time_seconds_count[5m]) > 1
  for: 5m
  labels:
    severity: warning
  annotations:
    summary: "High response time for {{ $labels.service_name }}"
    description: "{{ $labels.service_name }} response time is above 1 second"
```
