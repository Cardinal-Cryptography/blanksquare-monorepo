.PHONY: format-rust
format-rust: # Format all rust crates
	cargo fmt --all -- --check

.PHONY: lint-rust
lint-rust: # Lint all rust crates
	cargo clippy --release -- -D warnings

run:
	RUST_LOG=info cargo run --release
