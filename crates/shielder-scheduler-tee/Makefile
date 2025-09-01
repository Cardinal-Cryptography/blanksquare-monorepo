include .env

.PHONY: format-rust
format-rust:
	cargo fmt --all -- --check

.PHONY: lint-rust
lint-rust:
	cargo clippy --release -- -D warnings

.PHONY: test
test:
	cargo check --all-features

run-without-attestation:
	RUST_LOG=info \
	PUBLIC_KEY=$(PUBLIC_KEY) \
	PRIVATE_KEY=$(PRIVATE_KEY) \
	cargo run --release --features without_attestation
