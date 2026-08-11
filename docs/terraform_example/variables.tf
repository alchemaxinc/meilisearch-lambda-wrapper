variable "service_name" {
  description = "The generic name of the service we are deploying, used for tagging and naming resources"
  type        = string
}

variable "environment" {
  description = "The environment name (either 'development' or 'production')"
  type        = string

  validation {
    condition     = contains(["development", "production"], var.environment)
    error_message = "The environment value must be either 'development' or 'production'."
  }
}

variable "git_sha" {
  description = "The version of the service being deployed"
  type        = string
}

variable "ecr_repository_name" {
  description = "The name of the ECR repository where the Docker image is stored"
  type        = string
}

variable "api_lambda_timeout_seconds" {
  description = "Timeout (seconds) for the API Lambda (must be <= 29 due to API Gateway hard limit)."
  type        = number
  default     = 29

  validation {
    # API Gateway REST APIs hard-cap the integration timeout at 29 seconds and
    # cannot be configured higher. A Lambda timeout beyond that is silently
    # useless: API Gateway returns a 504 to the client at 29s regardless of
    # how long the Lambda keeps running, even if the request would have
    # succeeded. Keep this in sync with the timeout_milliseconds set on
    # aws_api_gateway_integration in api_gateway_rest.tf.
    condition     = var.api_lambda_timeout_seconds <= 29 && var.api_lambda_timeout_seconds >= 1 && var.api_lambda_timeout_seconds == floor(var.api_lambda_timeout_seconds)
    error_message = "api_lambda_timeout_seconds must be a whole number between 1 and 29 seconds (API Gateway integration timeout hard limit)."
  }
}

variable "meilisearch_master_key" {
  description = "The Meilisearch master key for authentication"
  type        = string
  sensitive   = true
}

variable "meilisearch_poll_interval_ms" {
  description = "Polling interval in milliseconds for checking Meilisearch task status"
  type        = number
  default     = 100

  validation {
    # Passed through as MEILISEARCH_POLL_INTERVAL_MS and parsed as a u64 by
    # the wrapper; a fractional value would fail that parse and panic at
    # startup, so require a whole number here instead.
    condition     = var.meilisearch_poll_interval_ms >= 1 && var.meilisearch_poll_interval_ms <= 5000 && var.meilisearch_poll_interval_ms == floor(var.meilisearch_poll_interval_ms)
    error_message = "meilisearch_poll_interval_ms must be a whole number between 1 and 5000 milliseconds."
  }
}
