SHELL := /bin/bash

# Service name (used for Docker image naming)
SERVICE_NAME=meilisearch-lambda-wrapper

# Build settings
DOCKER_IMAGE_NAME=$(SERVICE_NAME)-api
DOCKER_IMAGE_TAG?=abc123def

# Rust crate manifest paths
WRAPPER_MANIFEST=wrapper/Cargo.toml
SYNC_VERSIONS_MANIFEST=infrastructure/sync_versions/Cargo.toml

# Integration test compose file
INTEGRATION_COMPOSE=wrapper/tests/docker-compose.yml

# Functions for reusable docker build commands
define docker_build
	docker buildx build \
	--provenance=false \
	--platform linux/$(1) \
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
		--manifest-path $(SYNC_VERSIONS_MANIFEST) \
		--all-targets \
		-- -D warnings
	cargo clippy \
		--manifest-path $(WRAPPER_MANIFEST) \
		--all-targets \
		-- -D warnings
	cargo +nightly fmt \
		--manifest-path $(SYNC_VERSIONS_MANIFEST) \
		-- --check
	cargo +nightly fmt \
		--manifest-path $(WRAPPER_MANIFEST) \
		-- --check
	npx prettier --check .

.PHONY: format
format: ## Format files
	cargo clippy \
		--manifest-path $(SYNC_VERSIONS_MANIFEST) \
		--all-targets \
		--fix --allow-dirty
	cargo clippy \
		--manifest-path $(WRAPPER_MANIFEST) \
		--all-targets \
		--fix --allow-dirty
	cargo +nightly fmt \
		--manifest-path $(SYNC_VERSIONS_MANIFEST)
	cargo +nightly fmt \
		--manifest-path $(WRAPPER_MANIFEST)
	npx prettier --write .

.PHONY: build
build: ## Build all Rust crates
	cargo build \
		--manifest-path $(SYNC_VERSIONS_MANIFEST) \
		--release
	cargo build \
		--manifest-path $(WRAPPER_MANIFEST) \
		--release

.PHONY: test-unit
test-unit: ## Run unit tests
	cargo test \
		--manifest-path $(SYNC_VERSIONS_MANIFEST)
	cargo test \
		--manifest-path $(WRAPPER_MANIFEST)

.PHONY: test-integration
test-integration: ## Run integration tests
	docker build -t $(DOCKER_IMAGE_NAME):test .
	docker compose -f $(INTEGRATION_COMPOSE) up -d --wait
	MEILI_MASTER_KEY=test-master-key-12345 cargo test \
		--manifest-path $(WRAPPER_MANIFEST) \
		--features integration \
		--test integration_test -- --test-threads=1; \
	exit_code=$$?; \
	docker compose -f $(INTEGRATION_COMPOSE) down; \
	exit $$exit_code

.PHONY: build-docker-api-amd64
build-docker-api-amd64: ## Build Docker image for API (amd64)
	$(call docker_build,amd64,--load,$(DOCKER_IMAGE_NAME):$(DOCKER_IMAGE_TAG))

.PHONY: build-docker-api-arm64
build-docker-api-arm64: ## Build Docker image for API (arm64)
	$(call docker_build,arm64,,$(DOCKER_IMAGE_NAME):$(DOCKER_IMAGE_TAG))
