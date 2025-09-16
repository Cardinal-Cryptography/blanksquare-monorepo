FROM rust:1.88-slim-bookworm as builder

WORKDIR /app

COPY . .

RUN cargo build --release -p shielder-prover-server

FROM ubuntu:jammy

WORKDIR /app

COPY --from=builder /app/target/release/shielder-prover-server .

EXPOSE 3000

ENTRYPOINT ["/app/shielder-prover-server"]
