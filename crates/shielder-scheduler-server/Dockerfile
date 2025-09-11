FROM rust:1.88-slim-bookworm as builder

RUN apt-get update && \
    apt-get install -y \
      curl \
      pkg-config \
      libssl-dev \
      build-essential && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY . .

RUN cargo build --release -p shielder-scheduler-server

FROM debian:bookworm-slim

RUN apt-get update && \
    apt-get install -y \
      libssl3 \
      ca-certificates && \
    rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/shielder-scheduler-server /usr/local/bin/
RUN chmod +x /usr/local/bin/shielder-scheduler-server

RUN useradd -r -s /bin/false appuser
USER appuser

EXPOSE 3000 3001

ENTRYPOINT ["shielder-scheduler-server"]
