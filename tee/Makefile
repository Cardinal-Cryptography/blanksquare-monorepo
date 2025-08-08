.PHONY: test
test:
	cargo check --workspace

.PHONY: format-rust
format-rust: # Format all rust crates
	cargo fmt --all -- --check

.PHONY: lint-rust
lint-rust: # Lint all rust crates
	cargo clippy --release -- -D warnings
