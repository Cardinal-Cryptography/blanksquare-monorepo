# Multi-stage Dockerfile for shielder-cli
FROM rust:1.88-slim-bookworm AS builder

WORKDIR /app

# Install dependencies needed for building
RUN apt-get update && apt-get install -y \
    build-essential \
    pkg-config \
    libssl-dev \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Copy workspace manifests
COPY Cargo.toml Cargo.lock ./

# Copy all crates source code (needed for workspace dependencies)
COPY crates/ crates/

# Build shielder-cli binary
RUN cargo build --release -p shielder-cli

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Copy the binary from builder stage
COPY --from=builder /app/target/release/shielder-cli /usr/local/bin/shielder-cli

# Make binary executable
RUN chmod +x /usr/local/bin/shielder-cli

# Create a non-root user
RUN useradd -r -s /bin/false -d /home/shielder -m shielder

# Set the working directory to user's home
WORKDIR /home/shielder

# Switch to non-root user
USER shielder

# Set entrypoint
ENTRYPOINT ["shielder-cli"]
