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

# Build the fee-estimator binary
RUN cargo build --release -p fee-estimator

# Runtime stage
FROM debian:bookworm-slim

RUN apt-get update && \
    apt-get install -y \
      libssl3 \
      tini \
      ca-certificates && \
      rm -rf /tmp/* /var/{tmp,cache}/* /var/lib/{apt,dpkg}/

WORKDIR /app

# Copy the built binary from builder stage
COPY --from=builder /app/target/release/fee-estimator /usr/local/bin/fee-estimator
RUN chmod +x /usr/local/bin/fee-estimator

# Copy resources directory for ptau files
COPY --from=builder /app/resources /app/resources

# Create data directory with proper permissions
RUN mkdir -p /app/data && \
    useradd server && \
    chown -R server:server /app

USER server
ENV FEE_ESTIMATOR_DATA_DIR=/app/data
ENV PTAU_RESOURCES_DIR=/app/resources

ENTRYPOINT ["tini", "--", "fee-estimator"]
