FROM rust:1.88 AS builder

WORKDIR /app

COPY . .

RUN cargo build --release -p shielder-prover-server

FROM ubuntu:jammy

WORKDIR /app

COPY --from=builder /app/target/release/shielder-prover-server .

# Expose the default public port
EXPOSE 3000

ENTRYPOINT ["/app/shielder-prover-server"]
