.PHONY: format-rust
format-rust:
	cargo fmt --all -- --check

.PHONY: lint-rust
lint-rust:
	cargo clippy --release -- -D warnings

.PHONY: test
test:
	cargo check --all-features
