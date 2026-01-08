locals {
  my_synchronous_meilisearch_api_roles = [
    "arn:aws:iam::aws:policy/service-role/AWSLambdaBasicExecutionRole",
    "arn:aws:iam::aws:policy/service-role/AWSLambdaVPCAccessExecutionRole",
  ]
}

// Inline policy for EFS access
resource "aws_iam_role_policy" "my_synchronous_meilisearch_api_efs" {
  name = "${var.service_name}-api-efs-${var.environment}"
  role = aws_iam_role.my_synchronous_meilisearch_api.id

  policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Effect = "Allow"
        Action = [
          "elasticfilesystem:ClientMount",
          "elasticfilesystem:ClientWrite",
          "elasticfilesystem:ClientRootAccess",
        ]
        Resource = aws_efs_file_system.my_synchronous_meilisearch.arn
      },
    ]
  })
}

// Inline policy for ECR read access
resource "aws_iam_role_policy" "my_synchronous_meilisearch_api_ecr" {
  name = "${var.service_name}-api-ecr-${var.environment}"
  role = aws_iam_role.my_synchronous_meilisearch_api.id

  policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Effect = "Allow"
        Action = [
          "ecr:GetDownloadUrlForLayer",
          "ecr:BatchGetImage",
        ]
        Resource = aws_ecr_repository.my_synchronous_meilisearch_api.arn
      },
      {
        Effect = "Allow"
        Action = [
          "ecr:GetAuthorizationToken",
        ]
        Resource = "*"
      },
    ]
  })
}

resource "aws_iam_role" "my_synchronous_meilisearch_api" {
  name = "${var.service_name}-api-${var.environment}"

  assume_role_policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Action = "sts:AssumeRole"
        Effect = "Allow"
        Principal = {
          Service = "lambda.amazonaws.com"
        }
      },
    ]
  })
}

resource "aws_iam_role_policy_attachment" "my_synchronous_meilisearch_api_policies" {
  for_each = { for idx, role in local.my_synchronous_meilisearch_api_roles : idx => role }

  role       = aws_iam_role.my_synchronous_meilisearch_api.name
  policy_arn = each.value
}
