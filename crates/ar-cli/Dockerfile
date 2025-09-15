# Builder stage
FROM rust:1.88 AS builder

WORKDIR /app

# Copy workspace files for dependency resolution
COPY Cargo.toml Cargo.lock ./
COPY rust-toolchain.toml ./

# Copy the entire source code (needed for workspace builds)
COPY . .

# Build the ar-cli binary
RUN cargo build --release -p ar-cli

# Runtime stage
FROM debian:bullseye-slim

RUN apt-get update && \
    apt-get install -y \
      libssl-dev \
      ca-certificates && \
      rm -rf /tmp/* /var/{tmp,cache}/* /var/lib/{apt,dpkg}/

WORKDIR /app

# Copy the built binary from builder stage
COPY --from=builder /app/target/release/ar-cli /usr/local/bin/ar-cli
RUN chmod +x /usr/local/bin/ar-cli

ENTRYPOINT ["ar-cli"]
