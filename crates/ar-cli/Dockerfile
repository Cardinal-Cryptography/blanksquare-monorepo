# Builder stage
FROM rust:1.88-slim-bookworm as builder

RUN apt-get update && \
    apt-get install -y \
      curl \
      pkg-config \
      libssl-dev \
      build-essential && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy the entire source code (needed for workspace builds)
COPY . .

# Build the ar-cli binary
RUN cargo build --release -p ar-cli

# Runtime stage
FROM debian:bookworm-slim

RUN apt-get update && \
    apt-get install -y \
      libssl3 \
      ca-certificates && \
      rm -rf /tmp/* /var/{tmp,cache}/* /var/lib/{apt,dpkg}/

WORKDIR /app

# Copy the built binary from builder stage
COPY --from=builder /app/target/release/ar-cli /usr/local/bin/ar-cli
RUN chmod +x /usr/local/bin/ar-cli

ENTRYPOINT ["ar-cli"]
