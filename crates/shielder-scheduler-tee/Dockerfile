FROM ubuntu:22.04 AS builder

WORKDIR /app

# Install dependencies needed for Rust and building
RUN apt-get update && apt-get install -y \
    build-essential \
    curl \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Install Rust using rustup
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"

COPY . .
RUN cargo build --release -p shielder-scheduler-tee

FROM ubuntu:22.04

RUN apt-get update && apt-get install -y \
    ca-certificates \
    tini \
    && rm -rf /var/lib/apt/lists/*

COPY ./crates/shielder-scheduler-tee/artifacts/kms/kmstool_enclave_cli /usr/local/bin/
COPY ./crates/shielder-scheduler-tee/artifacts/kms/libnsm.so /usr/lib/x86_64-linux-gnu/

COPY --from=builder /app/target/release/shielder-scheduler-tee /usr/local/bin
RUN chmod +x /usr/local/bin/shielder-scheduler-tee

ENV RUST_LOG=debug

RUN useradd -r -s /bin/false server
USER server

ENTRYPOINT ["tini", "--", "shielder-scheduler-tee"]
