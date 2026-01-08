# Add Lambda permission for API Gateway to trigger the function
resource "aws_lambda_permission" "api_gateway_lambda_permission" {
  statement_id  = "AllowExecutionFromAPIGateway"
  action        = "lambda:InvokeFunction"
  function_name = aws_lambda_function.my_synchronous_meilisearch_api.function_name
  principal     = "apigateway.amazonaws.com"

  # This allows invocation from any stage, method and resource path within this API Gateway
  source_arn = "${aws_api_gateway_rest_api.my_synchronous_meilisearch_api_gateway.execution_arn}/*/*/*"
}

# Lambda function creation with Docker image
resource "aws_lambda_function" "my_synchronous_meilisearch_api" {
  function_name = "${var.service_name}-api-${var.environment}"
  role          = aws_iam_role.my_synchronous_meilisearch_api.arn

  image_uri    = "${aws_ecr_repository.my_synchronous_meilisearch_api.repository_url}:${local.bootstrap_tag}"
  package_type = "Image"

  architectures = ["arm64"]

  # Going below this will make Meilisearch unstable
  memory_size = 512

  timeout = var.api_lambda_timeout_seconds

  ephemeral_storage {
    size = 512
  }

  # Attach EFS to Lambda
  vpc_config {
    subnet_ids         = data.aws_subnets.default.ids
    security_group_ids = [data.aws_security_group.default.id]
  }

  file_system_config {
    arn              = aws_efs_access_point.my_synchronous_meilisearch.arn
    local_mount_path = local.efs_mount_path
  }

  environment {
    variables = {
      # General
      LOG_LEVEL   = "DEBUG"
      ENVIRONMENT = var.environment
      VERSION     = var.git_sha

      # AWS
      AWS_LAMBDA_TIMEOUT_SECONDS = tostring(var.api_lambda_timeout_seconds)

      # The AWS_LWA_READINESS_CHECK_PATH and AWS_LWA_READINESS_CHECK_PORT environment variables to let the AWS Lambda
      # Web Extension poll the Meilisearch endpoint http://localhost:7700/health until receiving an HTTP 200 OK
      # (cold start). From that point the Lambda will be considered ready to receive HTTP requests.
      AWS_LWA_PORT : "8080",
      AWS_LWA_READINESS_CHECK_PATH : "/health",
      AWS_LWA_READINESS_CHECK_PORT : "7700",

      # Meilisearch environment variables to make Meilisearch write on the EFS attached to the Lambda. The Meilisearch
      # database, dumps and snapshots are directed under the EFS mount path which corresponds to the root directory
      # of our access point. This ensures that documents indexed in Meilisearch are persisted between invocations of
      # the Lambda.
      MEILI_ENV : var.environment
      MEILI_EXPERIMENTAL_LOGS_MODE : "json",
      MEILI_NO_ANALYTICS : "true",
      MEILI_MASTER_KEY : var.meilisearch_master_key,
      MEILI_DB_PATH : "${local.efs_mount_path}/data",
      MEILI_DUMP_DIR : "${local.efs_mount_path}/dump",
      MEILI_SNAPSHOT_DIR : "${local.efs_mount_path}/snapshot",
      MEILI_EXPERIMENTAL_MAX_NUMBER_OF_BATCHED_TASKS : 1,

      # Wrapper configuration for synchronous write operations
      MEILISEARCH_POLL_INTERVAL_MS : tostring(var.meilisearch_poll_interval_ms)
    }
  }

  logging_config {
    log_format = "Text"
    log_group  = "/aws/lambda/my-synchronous-meilisearch-api-${var.environment}"
  }

  # CRITICAL: Ignore future changes to image_uri so Terraform doesn't
  # overwrite your real app deployments later when you manually push new images
  lifecycle {
    ignore_changes = [image_uri]
  }

  depends_on = [
    aws_ecr_repository.my_synchronous_meilisearch_api,
    aws_iam_role_policy.my_synchronous_meilisearch_api_efs,
    null_resource.ecr_bootstrap_image,
    aws_efs_mount_target.my_synchronous_meilisearch,
  ]
}
