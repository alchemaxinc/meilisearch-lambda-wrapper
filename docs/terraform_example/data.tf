data "aws_region" "current" {}

data "aws_caller_identity" "current" {}

# Get available subnets from default VPC
data "aws_subnets" "default" {
  filter {
    name   = "vpc-id"
    values = [data.aws_vpc.default.id]
  }
}

data "aws_vpc" "default" {
  default = true
}

# Get the default security group for the default VPC
# This allows all traffic within the VPC, which is fine for internal Lambda/EFS communication
data "aws_security_group" "default" {
  vpc_id = data.aws_vpc.default.id
  name   = "default"
}
