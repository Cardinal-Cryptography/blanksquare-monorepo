.PHONY: format-rust
format-rust:
	cargo fmt --all -- --check

.PHONY: lint-rust
lint-rust:
	cargo clippy --release -- -D warnings
	cargo clippy --release --features local-run -- -D warnings

.PHONY: build
build:
	cargo build --release
	cargo build --release --features local-run

.PHONY: test
test:
	cargo check
	cargo check --features local-run
