output "lambda_api_timeout_seconds" {
  description = "The timeout setting for the API Lambda function in seconds"
  value       = aws_lambda_function.my_synchronous_meilisearch_api.timeout
}

output "api_gateway_invoke_url" {
  description = "The invoke URL for the API Gateway"
  value       = aws_api_gateway_stage.my_synchronous_meilisearch_api_gateway_stage.invoke_url
}
