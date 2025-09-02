FROM rust:1.89-slim-bookworm AS builder

RUN apt-get update && \
    apt-get install -y \
      curl \
      libssl-dev \
      ca-certificates \
      build-essential \
      pkg-config \
      openssh-client && \
    rm -rf /tmp/* /var/{tmp,cache}/* /var/lib/{apt,dpkg}/

WORKDIR /app

COPY . .

RUN cd crates/shielder-scheduler-tee && cargo build --release --features without_attestation

FROM debian:bookworm-slim

RUN apt-get update && \
    apt-get install -y \
      libssl-dev \
      tini \
      ca-certificates && \
      rm -rf /tmp/* /var/{tmp,cache}/* /var/lib/{apt,dpkg}/

COPY --from=builder /app/target/release/shielder-scheduler-tee /usr/local/bin
RUN chmod +x /usr/local/bin/shielder-scheduler-tee

RUN useradd server
USER server

ENTRYPOINT ["tini", "--", "shielder-scheduler-tee"]
