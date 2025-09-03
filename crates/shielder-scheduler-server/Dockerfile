FROM rust:1.81-slim-bookworm as builder

RUN apt-get update && \
    apt-get install -y \
      curl \
      pkg-config \
      libssl-dev \
      build-essential && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY . .

# Build the scheduler server
RUN cd crates/shielder-scheduler-server && cargo build --release

FROM debian:bookworm-slim

RUN apt-get update && \
    apt-get install -y \
      libssl3 \
      ca-certificates && \
    rm -rf /var/lib/apt/lists/*

COPY --from=aws-nitro-enclaves-cli:latest /usr/bin/kmstool_enclave_cli /usr/bin/kmstool_enclave_cli
RUN /usr/bin/kmstool_enclave_cli --help || true

COPY --from=builder /app/target/release/shielder-scheduler-server /usr/local/bin/
RUN chmod +x /usr/local/bin/shielder-scheduler-server

RUN useradd -r -s /bin/false appuser
USER appuser

EXPOSE 3000 3001

ENTRYPOINT ["shielder-scheduler-server"]
