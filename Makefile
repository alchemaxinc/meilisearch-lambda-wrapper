SHELL := /bin/bash

# Service name (used for Docker image naming)
SERVICE_NAME=meilisearch-lambda-wrapper

# Build settings
DOCKER_IMAGE_NAME=$(SERVICE_NAME)-api
DOCKER_IMAGE_TAG?=abc123def

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
	terraform -chdir=infrastructure/terraform fmt -check
	black --check .
	npx prettier --check .

.PHONY: format
format: ## Format files
	black .
	npx prettier --write .

.PHONY: test-integration
test-integration: ## Run integration tests
	docker compose -f wrapper/test/docker-compose.yml build
	docker compose -f wrapper/test/docker-compose.yml up --abort-on-container-exit --exit-code-from integration-tests

.PHONY: build-docker-api-amd64
build-docker-api-amd64: ## Build Docker image for API (amd64)
	$(call docker_build,amd64,--load,$(DOCKER_IMAGE_NAME):$(DOCKER_IMAGE_TAG))

.PHONY: build-docker-api-arm64
build-docker-api-arm64: ## Build Docker image for API (arm64)
	$(call docker_build,arm64,,$(DOCKER_IMAGE_NAME):$(DOCKER_IMAGE_TAG))
