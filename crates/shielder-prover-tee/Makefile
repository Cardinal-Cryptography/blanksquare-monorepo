.PHONY: format-rust
format-rust:
	cargo fmt --all -- --check

.PHONY: lint-rust
lint-rust:
	cargo clippy --release -- -D warnings

.PHONY: build
build:
	cargo build --release

.PHONY: test
test:
	cargo test --release --all-features
