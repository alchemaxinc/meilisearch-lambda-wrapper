terraform {
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
