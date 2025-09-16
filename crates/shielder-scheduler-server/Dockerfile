FROM rust:1.88-slim-bookworm as builder

# Install build dependencies including OpenSSL dev libraries
# Note: libssl-dev is needed for compilation but the final binary uses RustLS,
# so OpenSSL is not required at runtime (binary uses pure Rust crypto)
RUN apt-get update && \
    apt-get install -y \
      curl \
      pkg-config \
      libssl-dev \
      build-essential && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY . .

RUN cargo build --release -p shielder-scheduler-server

FROM debian:bookworm-slim
# Runtime image doesn't need OpenSSL - the binary uses RustLS with statically linked crypto
COPY --from=builder /app/target/release/shielder-scheduler-server /usr/local/bin/

RUN chmod +x /usr/local/bin/shielder-scheduler-server

RUN useradd -r -s /bin/false appuser
USER appuser

EXPOSE 3000 3001

ENTRYPOINT ["shielder-scheduler-server"]
