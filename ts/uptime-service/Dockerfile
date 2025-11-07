FROM oven/bun:1.1.38-slim AS builder

WORKDIR /app

# Copy package files
COPY package.json bun.lock* ./

# Install dependencies
RUN bun install --frozen-lockfile --production

# Copy source code
COPY src ./src

FROM oven/bun:1.1.38-slim

WORKDIR /app

# Install ca-certificates for HTTPS requests
RUN apt-get update && \
    apt-get install -y ca-certificates && \
    rm -rf /var/lib/apt/lists/*

# Copy dependencies and source from builder
COPY --from=builder /app/node_modules ./node_modules
COPY --from=builder /app/package.json ./package.json
COPY --from=builder /app/src ./src

# Create non-root user
RUN useradd -r -s /bin/false appuser && \
    chown -R appuser:appuser /app

USER appuser

# Expose metrics port
EXPOSE 9615

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
  CMD bun run -e "fetch('http://localhost:9615/health').then(r => r.ok ? process.exit(0) : process.exit(1)).catch(() => process.exit(1))"

ENTRYPOINT ["bun", "run", "src/index.js"]
