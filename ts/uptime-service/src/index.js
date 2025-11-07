/**
 * Uptime Service - Main Entry Point
 * Monitors health endpoints and exposes Prometheus metrics
 */

import { loadConfig } from "./config.js";
import { startMetricsServer } from "./server.js";
import { startProbing } from "./prober.js";

// Main function
async function main() {
  console.log("=== Uptime Monitoring Service ===\n");

  try {
    // Load and validate configuration
    const config = loadConfig();

    // Start HTTP server for Prometheus metrics
    startMetricsServer(config.port);

    // Start health check probes
    startProbing(config);
  } catch (error) {
    console.error("Fatal error:", error.message);
    process.exit(1);
  }
}

// Run the service
main();
