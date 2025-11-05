/**
 * Configuration loader - reads and validates environment variables
 */

export function loadConfig() {
  // Load environment variables (Bun automatically loads .env)
  const config = {
    port: parseInt(process.env.PORT || "9090", 10),
    probeInterval: parseInt(process.env.PROBE_INTERVAL || "30000", 10),
    timeout: parseInt(process.env.TIMEOUT || "5000", 10),
    endpoints: [],
  };

  // Parse and validate endpoints
  if (!process.env.ENDPOINTS) {
    console.error("ERROR: ENDPOINTS environment variable is required");
    process.exit(1);
  }

  try {
    const endpoints = JSON.parse(process.env.ENDPOINTS);

    if (!Array.isArray(endpoints)) {
      throw new Error("ENDPOINTS must be a JSON array");
    }

    if (endpoints.length === 0) {
      throw new Error("ENDPOINTS array cannot be empty");
    }

    // Validate each endpoint
    config.endpoints = endpoints.map((endpoint, index) => {
      if (!endpoint.name || typeof endpoint.name !== "string") {
        throw new Error(
          `Endpoint at index ${index} missing required 'name' field`
        );
      }
      if (!endpoint.url || typeof endpoint.url !== "string") {
        throw new Error(
          `Endpoint at index ${index} missing required 'url' field`
        );
      }

      return {
        name: endpoint.name,
        url: endpoint.url,
        method: endpoint.method || "GET",
        expectedStatus: endpoint.expectedStatus || 200,
      };
    });

    // Check for duplicate names
    const names = config.endpoints.map((e) => e.name);
    const duplicates = names.filter(
      (name, index) => names.indexOf(name) !== index
    );
    if (duplicates.length > 0) {
      throw new Error(
        `Duplicate endpoint names found: ${duplicates.join(", ")}`
      );
    }
  } catch (error) {
    console.error("ERROR: Failed to parse ENDPOINTS:", error.message);
    process.exit(1);
  }

  // Validate config values
  if (config.port < 1 || config.port > 65535) {
    console.error("ERROR: PORT must be between 1 and 65535");
    process.exit(1);
  }

  if (config.probeInterval < 1000) {
    console.error("ERROR: PROBE_INTERVAL must be at least 1000ms");
    process.exit(1);
  }

  if (config.timeout < 100) {
    console.error("ERROR: TIMEOUT must be at least 100ms");
    process.exit(1);
  }

  return config;
}
