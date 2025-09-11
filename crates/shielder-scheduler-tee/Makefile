.PHONY: format-rust
format-rust:
	cargo fmt --all -- --check

.PHONY: lint-rust
lint-rust:
	cargo clippy --release -- -D warnings

.PHONY: test
test:
	@echo "Testing default build (production mode with attestation)..."
	cargo check --release
	@echo "Testing local-run build (development mode without attestation)..."
	cargo check --release --features local-run
	@echo "All builds tested successfully!"

.PHONY: build-all
build-all:
	@echo "Building default configuration (production)..."
	cargo build --release
	@echo "Building local-run configuration (development)..."
	cargo build --release --features local-run
	@echo "All builds completed successfully!"
