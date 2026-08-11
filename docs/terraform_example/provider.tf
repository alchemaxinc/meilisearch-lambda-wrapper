terraform {
  # use_lockfile in backend.tf (native S3 locking) requires Terraform 1.11+.
  required_version = ">= 1.11"

  required_providers {
    aws = {
      source  = "hashicorp/aws"
      version = "~> 6.1"
    }
  }
}

provider "aws" {
  region = "eu-north-1"

  default_tags {
    tags = {
      Environment = var.environment
      Project     = var.service_name
    }
  }
}
