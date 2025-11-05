/**
 * Health probe logic - performs periodic health checks on configured endpoints
 */

import { recordSuccess, recordFailure } from "./metrics.js";

/**
 * Perform a single health check on an endpoint
 */
async function probeEndpoint(endpoint, timeout) {
  const startTime = Date.now();

  try {
    const controller = new AbortController();
    const timeoutId = setTimeout(() => controller.abort(), timeout);

    const response = await fetch(endpoint.url, {
      method: endpoint.method,
      signal: controller.signal,
      headers: {
        "User-Agent": "uptime-service/1.0"
      }
    });

    clearTimeout(timeoutId);

    const endTime = Date.now();
    const responseTime = (endTime - startTime) / 1000; // Convert to seconds

    // Check if status code matches expected
    if (response.status === endpoint.expectedStatus) {
      recordSuccess(endpoint.name, endpoint.url, responseTime);
      console.log(
        `[${new Date().toISOString()}] ✓ ${endpoint.name} - UP (${
          response.status
        }, ${responseTime.toFixed(3)}s)`
      );
      return true;
    } else {
      recordFailure(endpoint.name, endpoint.url);
      console.log(
        `[${new Date().toISOString()}] ✗ ${endpoint.name} - DOWN (status: ${
          response.status
        }, expected: ${endpoint.expectedStatus})`
      );
      return false;
    }
  } catch (error) {
    recordFailure(endpoint.name, endpoint.url);

    let errorMessage = error.message;
    if (error.name === "AbortError") {
      errorMessage = "timeout";
    }

    console.log(
      `[${new Date().toISOString()}] ✗ ${
        endpoint.name
      } - DOWN (${errorMessage})`
    );
    return false;
  }
}

/**
 * Probe all configured endpoints
 */
async function probeAllEndpoints(endpoints, timeout) {
  const promises = endpoints.map((endpoint) =>
    probeEndpoint(endpoint, timeout)
  );
  await Promise.all(promises);
}

/**
 * Start the probe scheduler
 */
export function startProbing(config) {
  console.log("\n=== Starting uptime monitoring ===");
  console.log(`Monitoring ${config.endpoints.length} endpoint(s):`);
  config.endpoints.forEach((endpoint) => {
    console.log(`  - ${endpoint.name}: ${endpoint.url}`);
  });
  console.log(`Probe interval: ${config.probeInterval}ms`);
  console.log(`Timeout: ${config.timeout}ms`);
  console.log("================================\n");

  // Perform initial probe immediately
  probeAllEndpoints(config.endpoints, config.timeout);

  // Schedule periodic probes
  const intervalId = setInterval(() => {
    probeAllEndpoints(config.endpoints, config.timeout);
  }, config.probeInterval);

  // Handle graceful shutdown
  process.on("SIGINT", () => {
    console.log("\n\nShutting down gracefully...");
    clearInterval(intervalId);
    process.exit(0);
  });

  process.on("SIGTERM", () => {
    console.log("\n\nShutting down gracefully...");
    clearInterval(intervalId);
    process.exit(0);
  });

  return intervalId;
}
