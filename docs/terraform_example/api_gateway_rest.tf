resource "aws_api_gateway_rest_api" "my_synchronous_meilisearch_api_gateway" {
  name = "${var.service_name}-api-gateway-${var.environment}"

  endpoint_configuration {
    types = [
      "REGIONAL"
    ]
  }
}

# API Gateway Resource
resource "aws_api_gateway_resource" "my_synchronous_meilisearch_api_gateway_resource" {
  path_part   = "{myProxy+}"
  parent_id   = aws_api_gateway_rest_api.my_synchronous_meilisearch_api_gateway.root_resource_id
  rest_api_id = aws_api_gateway_rest_api.my_synchronous_meilisearch_api_gateway.id
}

resource "aws_api_gateway_method_response" "my_synchronous_meilisearch_api_gateway_method_response" {
  rest_api_id = aws_api_gateway_rest_api.my_synchronous_meilisearch_api_gateway.id
  resource_id = aws_api_gateway_resource.my_synchronous_meilisearch_api_gateway_resource.id
  http_method = aws_api_gateway_method.my_synchronous_meilisearch_api_gateway_method_options.http_method
  status_code = "200"

  response_parameters = {
    "method.response.header.Access-Control-Allow-Origin"  = true
    "method.response.header.Access-Control-Allow-Methods" = true
    "method.response.header.Access-Control-Allow-Headers" = true
  }
}

# API Gateway Method (ANY), on the resource
resource "aws_api_gateway_method" "my_synchronous_meilisearch_api_gateway_method_any" {
  rest_api_id   = aws_api_gateway_rest_api.my_synchronous_meilisearch_api_gateway.id
  resource_id   = aws_api_gateway_resource.my_synchronous_meilisearch_api_gateway_resource.id
  http_method   = "ANY"
  authorization = "NONE"

  # Add request parameter for the proxy path
  request_parameters = {
    "method.request.path.myProxy" = true
  }
}

resource "aws_api_gateway_integration" "my_synchronous_meilisearch_api_gateway_integration_any" {
  rest_api_id             = aws_api_gateway_rest_api.my_synchronous_meilisearch_api_gateway.id
  resource_id             = aws_api_gateway_resource.my_synchronous_meilisearch_api_gateway_resource.id
  http_method             = aws_api_gateway_method.my_synchronous_meilisearch_api_gateway_method_any.http_method
  integration_http_method = "POST"
  type                    = "AWS_PROXY"
  uri                     = aws_lambda_function.my_synchronous_meilisearch_api.invoke_arn
}

# API Gateway Method (OPTIONS), on the resource. A mock-resource with no real backend, just to handle CORS
resource "aws_api_gateway_method" "my_synchronous_meilisearch_api_gateway_method_options" {
  rest_api_id   = aws_api_gateway_rest_api.my_synchronous_meilisearch_api_gateway.id
  resource_id   = aws_api_gateway_resource.my_synchronous_meilisearch_api_gateway_resource.id
  http_method   = "OPTIONS"
  authorization = "NONE"
}

resource "aws_api_gateway_integration" "my_synchronous_meilisearch_api_gateway_integration_options" {
  rest_api_id = aws_api_gateway_rest_api.my_synchronous_meilisearch_api_gateway.id
  resource_id = aws_api_gateway_resource.my_synchronous_meilisearch_api_gateway_resource.id
  http_method = aws_api_gateway_method.my_synchronous_meilisearch_api_gateway_method_options.http_method
  type        = "MOCK"

  request_templates = {
    "application/json" = "{\"statusCode\": 200}"
  }
}

resource "aws_api_gateway_integration_response" "my_synchronous_meilisearch_api_gateway_integration_options_response" {
  rest_api_id = aws_api_gateway_rest_api.my_synchronous_meilisearch_api_gateway.id
  resource_id = aws_api_gateway_resource.my_synchronous_meilisearch_api_gateway_resource.id
  http_method = aws_api_gateway_method.my_synchronous_meilisearch_api_gateway_method_options.http_method
  status_code = aws_api_gateway_method_response.my_synchronous_meilisearch_api_gateway_method_response.status_code

  depends_on = [
    aws_api_gateway_integration.my_synchronous_meilisearch_api_gateway_integration_any,
    aws_api_gateway_integration.my_synchronous_meilisearch_api_gateway_integration_options,
  ]

  response_parameters = {
    "method.response.header.Access-Control-Allow-Origin"  = "'*'"
    "method.response.header.Access-Control-Allow-Methods" = "'DELETE,GET,HEAD,OPTIONS,PATCH,POST,PUT'"
    # Include both common case and lowercase to avoid any case-sensitivity quirks in browsers or intermediaries
    "method.response.header.Access-Control-Allow-Headers" = "'Content-Type,X-Amz-Date,Authorization,X-Api-Key,X-Amz-Security-Token,X-Meilisearch-Client,x-meilisearch-client'"
  }
}

resource "aws_api_gateway_deployment" "my_synchronous_meilisearch_api_gateway_deployment" {
  rest_api_id = aws_api_gateway_rest_api.my_synchronous_meilisearch_api_gateway.id

  triggers = {
    redeployment = sha1(jsonencode([
      aws_api_gateway_rest_api.my_synchronous_meilisearch_api_gateway.body,
      aws_api_gateway_integration.my_synchronous_meilisearch_api_gateway_integration_any.id,
      aws_api_gateway_integration.my_synchronous_meilisearch_api_gateway_integration_options.id,
      # Also hash response parameter maps so changes to CORS headers force a new deployment
      aws_api_gateway_integration_response.my_synchronous_meilisearch_api_gateway_integration_options_response.response_parameters,
      aws_api_gateway_method.my_synchronous_meilisearch_api_gateway_method_any.id,
      aws_api_gateway_method.my_synchronous_meilisearch_api_gateway_method_options.id,
      aws_api_gateway_method_response.my_synchronous_meilisearch_api_gateway_method_response.response_parameters,
      aws_api_gateway_method.my_synchronous_meilisearch_api_gateway_method_any.request_parameters,
      aws_api_gateway_method.my_synchronous_meilisearch_api_gateway_method_options.request_parameters
    ]))
  }

  lifecycle {
    create_before_destroy = true
  }
}

resource "aws_api_gateway_stage" "my_synchronous_meilisearch_api_gateway_stage" {
  stage_name    = "default"
  rest_api_id   = aws_api_gateway_rest_api.my_synchronous_meilisearch_api_gateway.id
  deployment_id = aws_api_gateway_deployment.my_synchronous_meilisearch_api_gateway_deployment.id
}
