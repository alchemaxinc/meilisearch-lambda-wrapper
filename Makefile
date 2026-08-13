SHELL := /bin/bash

# Service name (used for Docker image naming)
SERVICE_NAME=meilisearch-lambda-wrapper

# Build settings
DOCKER_IMAGE_NAME=$(SERVICE_NAME)-api
DOCKER_IMAGE_TAG?=abc123def

# Sourced from rust-toolchain.toml so the cargo updater drives the Docker
# builder image version too (see .github/workflows/update-deps-docker.yml,
# which excludes `rust` from the docker-image bumper).
RUST_VERSION := $(shell sed -nE 's/^channel[[:space:]]*=[[:space:]]*"([^"]+)".*/\1/p' rust-toolchain.toml)

# Rust crate manifest paths
WRAPPER_MANIFEST=wrapper/Cargo.toml

STRESS_TEST_SCRIPT=infrastructure/stress_tests/stress-test.js

INTEGRATION_COMPOSE=wrapper/tests/docker-compose.yml

# Functions for reusable docker build commands
define docker_build
	docker buildx build \
	--provenance=false \
	--platform linux/$(1) \
	--build-arg RUST_VERSION=$(RUST_VERSION) \
	$(2) \
	-t $(3) \
	-f Dockerfile .
endef

.PHONY: help
help: ## Show this help
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | \
	sort | awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-35s\033[0m %s\n", $$1, $$2}'

.PHONY: clean
clean: ## Clean up built files
	docker rmi $(DOCKER_IMAGE_NAME):$(DOCKER_IMAGE_TAG) || true

.PHONY: lint
lint: ## Run linter
	cargo clippy \
		--manifest-path $(WRAPPER_MANIFEST) \
		--all-targets \
		-- -D warnings
	cargo +nightly fmt \
		--manifest-path $(WRAPPER_MANIFEST) \
		-- --check
	npx prettier --check .

.PHONY: format
format: ## Format files
	cargo clippy \
		--manifest-path $(WRAPPER_MANIFEST) \
		--all-targets \
		--fix --allow-dirty
	cargo +nightly fmt \
		--manifest-path $(WRAPPER_MANIFEST)
	npx prettier --write .

.PHONY: build
build: ## Build all Rust crates
	cargo build \
		--manifest-path $(WRAPPER_MANIFEST) \
		--release

.PHONY: test-unit
test-unit: ## Run unit tests
	cargo test \
		--manifest-path $(WRAPPER_MANIFEST)

.PHONY: test-integration
test-integration: ## Run integration tests
	docker build --build-arg RUST_VERSION=$(RUST_VERSION) -t $(DOCKER_IMAGE_NAME):test .
	docker compose -f $(INTEGRATION_COMPOSE) up -d --wait
	MEILI_MASTER_KEY=test-master-key-12345 cargo test \
		--manifest-path $(WRAPPER_MANIFEST) \
		--features integration \
		--test integration_test -- --test-threads=1; \
	exit_code=$$?; \
	docker compose -f $(INTEGRATION_COMPOSE) down; \
	exit $$exit_code

.PHONY: test-stress
test-stress: ## Run k6 stress tests (requires k6: https://grafana.com/docs/k6/latest/set-up/install-k6/)
	docker build --build-arg RUST_VERSION=$(RUST_VERSION) -t $(DOCKER_IMAGE_NAME):test .
	docker compose -f $(INTEGRATION_COMPOSE) up -d --wait
	k6 run $(STRESS_TEST_SCRIPT); \
	exit_code=$$?; \
	docker compose -f $(INTEGRATION_COMPOSE) down; \
	exit $$exit_code

.PHONY: build-docker-api-amd64
build-docker-api-amd64: ## Build Docker image for API (amd64)
	$(call docker_build,amd64,--load,$(DOCKER_IMAGE_NAME):$(DOCKER_IMAGE_TAG))

.PHONY: build-docker-api-arm64
build-docker-api-arm64: ## Build Docker image for API (arm64)
	$(call docker_build,arm64,--load,$(DOCKER_IMAGE_NAME):$(DOCKER_IMAGE_TAG))
