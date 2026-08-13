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

# EFS allows only one mount target per Availability Zone per file system.
# The default VPC normally has exactly one subnet per AZ, but if it's ever
# customized to have more, `for_each`-ing over every subnet id directly
# (subnet_ids below) would try to create two mount targets in the same AZ,
# and the second `aws_efs_mount_target` would fail to apply. Fetching each
# subnet's AZ here lets efs.tf pick just one subnet per AZ instead.
data "aws_subnet" "default" {
  for_each = toset(data.aws_subnets.default.ids)
  id       = each.value
}

# Get the default security group for the default VPC
# This allows all traffic within the VPC, which is fine for internal Lambda/EFS communication
data "aws_security_group" "default" {
  vpc_id = data.aws_vpc.default.id
  name   = "default"
}
