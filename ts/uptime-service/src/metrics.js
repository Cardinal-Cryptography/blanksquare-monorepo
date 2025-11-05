/**
 * Prometheus metrics definitions
 */

import { Registry, Gauge, Counter, Histogram } from "prom-client";

// Create a new registry
export const register = new Registry();

// Metric: Service availability (1 = up, 0 = down)
export const serviceUp = new Gauge({
  name: "service_up",
  help: "Service availability status (1 = up, 0 = down)",
  labelNames: ["service_name", "endpoint"],
  registers: [register]
});

// Metric: Response time in seconds
export const serviceResponseTime = new Histogram({
  name: "service_response_time_seconds",
  help: "Service response time in seconds",
  labelNames: ["service_name", "endpoint"],
  buckets: [0.001, 0.01, 0.1, 0.5, 1, 2, 5, 10],
  registers: [register]
});

// Metric: Last probe timestamp
export const serviceLastProbeTimestamp = new Gauge({
  name: "service_last_probe_timestamp",
  help: "Unix timestamp of the last probe attempt",
  labelNames: ["service_name", "endpoint"],
  registers: [register]
});

/**
 * Record a successful probe
 */
export function recordSuccess(serviceName, endpoint, responseTime) {
  const labels = { service_name: serviceName, endpoint };

  serviceUp.set(labels, 1);
  serviceResponseTime.observe(labels, responseTime);
  serviceLastProbeTimestamp.set(labels, Date.now() / 1000);
}

/**
 * Record a failed probe
 */
export function recordFailure(serviceName, endpoint) {
  const labels = { service_name: serviceName, endpoint };

  serviceUp.set(labels, 0);
  serviceLastProbeTimestamp.set(labels, Date.now() / 1000);
}
