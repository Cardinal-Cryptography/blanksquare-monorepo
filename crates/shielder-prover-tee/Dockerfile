FROM rust:1.88-slim-bookworm as builder

WORKDIR /app

RUN apt-get update && apt-get install -y \
    build-essential \
    curl \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

COPY . .
RUN cargo build --release -p shielder-prover-tee

FROM ubuntu:22.04

RUN apt-get update && apt-get install -y \
    ca-certificates \
    tini \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/shielder-prover-tee /usr/local/bin
RUN chmod +x /usr/local/bin/shielder-prover-tee

ENV RUST_LOG=debug

RUN useradd -r -s /bin/false server
USER server

ENTRYPOINT ["tini", "--", "shielder-prover-tee"]
