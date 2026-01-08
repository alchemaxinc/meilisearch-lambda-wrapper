// ECR Repository for Lambda Docker image
resource "aws_ecr_repository" "my_synchronous_meilisearch_api" {
  name         = var.ecr_repository_name
  force_delete = true

  image_scanning_configuration {
    scan_on_push = true
  }

  encryption_configuration {
    encryption_type = "KMS"
  }

  tags = {
    Name = "${var.service_name}-api"
  }
}

// ECR lifecycle policy
resource "aws_ecr_lifecycle_policy" "my_synchronous_meilisearch_api" {
  repository = aws_ecr_repository.my_synchronous_meilisearch_api.name

  policy = jsonencode({
    rules = [
      {
        rulePriority = 1
        description  = "Keep last 10 images"
        selection = {
          tagStatus   = "any"
          countType   = "imageCountMoreThan"
          countNumber = 2
        }
        action = {
          type = "expire"
        }
      }
    ]
  })
}

// Bootstrap: Push a dummy placeholder image to ECR to resolve chicken-and-egg problem
// This allows the Lambda to be created without requiring a real application image first
resource "null_resource" "ecr_bootstrap_image" {
  triggers = {
    # Only run this if the ECR repo is re-created
    repo_url = aws_ecr_repository.my_synchronous_meilisearch_api.repository_url
  }

  provisioner "local-exec" {
    interpreter = ["/bin/bash", "-c"]
    command     = <<EOF
      # 1. Login to ECR
      aws ecr get-login-password --region ${data.aws_region.current.region} | \
        docker login --username AWS --password-stdin ${aws_ecr_repository.my_synchronous_meilisearch_api.repository_url}

      # 2. Pull a tiny public image
      docker pull hello-world:latest

      # 3. Tag it with your ECR repo URL
      docker tag hello-world:latest ${aws_ecr_repository.my_synchronous_meilisearch_api.repository_url}:${local.bootstrap_tag}

      # 4. Push it to ECR
      docker push ${aws_ecr_repository.my_synchronous_meilisearch_api.repository_url}:${local.bootstrap_tag}
    EOF
  }

  depends_on = [aws_ecr_repository.my_synchronous_meilisearch_api]
}

