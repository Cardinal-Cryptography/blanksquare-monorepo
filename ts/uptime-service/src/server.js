/**
 * HTTP server that exposes Prometheus metrics
 */

import express from "express";
import { register } from "./metrics.js";

/**
 * Create and start the metrics HTTP server
 */
export function startMetricsServer(port) {
  const app = express();

  // Health check endpoint for the service itself
  app.get("/health", (req, res) => {
    res.status(200).json({
      status: "ok",
      timestamp: new Date().toISOString(),
    });
  });

  // Prometheus metrics endpoint
  app.get("/metrics", async (req, res) => {
    try {
      res.set("Content-Type", register.contentType);
      const metrics = await register.metrics();
      res.send(metrics);
    } catch (error) {
      console.error("Error generating metrics:", error);
      res.status(500).send("Error generating metrics");
    }
  });

  // Root endpoint with service info
  app.get("/", (req, res) => {
    res.status(200).json({
      service: "uptime-service",
      version: "1.0.0",
      endpoints: {
        health: "/health",
        metrics: "/metrics",
      },
    });
  });

  // Start server
  const server = app.listen(port, () => {
    console.log(`Metrics server listening on http://localhost:${port}`);
    console.log(`  - Metrics endpoint: http://localhost:${port}/metrics`);
    console.log(`  - Health endpoint: http://localhost:${port}/health\n`);
  });

  // Handle server errors
  server.on("error", (error) => {
    if (error.code === "EADDRINUSE") {
      console.error(`ERROR: Port ${port} is already in use`);
    } else {
      console.error("Server error:", error);
    }
    process.exit(1);
  });

  return server;
}
