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
    condition     = var.api_lambda_timeout_seconds <= 29 && var.api_lambda_timeout_seconds >= 1
    error_message = "api_lambda_timeout_seconds must be between 1 and 29 seconds (API Gateway integration timeout hard limit)."
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
    condition     = var.meilisearch_poll_interval_ms > 0 && var.meilisearch_poll_interval_ms <= 5000
    error_message = "meilisearch_poll_interval_ms must be between 1 and 5000 milliseconds."
  }
}
