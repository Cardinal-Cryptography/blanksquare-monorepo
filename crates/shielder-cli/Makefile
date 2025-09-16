.PHONY: build
build: ## Build the project
	@cargo build --release

.PHONY: test
test: ## Run tests
	@cargo test --release

.PHONY: lint
lint: ## Run clippy and fmt
	@cargo clippy --release -- -D warnings
	@cargo +nightly fmt --all

.PHONY: build-image
build-image: ## Build Docker image for shielder-cli
	@if ! docker image inspect shielder-cli:latest >/dev/null 2>&1; then \
		echo "Building shielder-cli Docker image..."; \
		cd ../.. && docker build --tag "shielder-cli:latest" -f "crates/shielder-cli/Dockerfile" .; \
	else \
		echo "shielder-cli Docker image already exists, skipping build"; \
	fi

help: ## Displays this help
	@awk 'BEGIN {FS = ":.*##"; printf "$(MAKEFILE_NAME)\nUsage:\n  make \033[1;36m<target>\033[0m\n\nTargets:\n"} /^[a-zA-Z0-9_-]+:.*?##/ { printf "  \033[1;36m%-25s\033[0m %s\n", $$1, $$2 }' $(MAKEFILE_LIST)
