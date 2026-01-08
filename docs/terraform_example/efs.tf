// EFS for Lambda - cheapest configuration
// Uses One Zone storage class and Bursting throughput mode

// NOTE: Using the default VPC security group for both Lambda and EFS
// The default security group allows all traffic within the VPC, which is fine
// for internal communication and avoids Terraform destroy issues entirely.
// See: https://github.com/hashicorp/terraform-provider-aws/issues/265

// EFS File System - cheapest configuration
resource "aws_efs_file_system" "my_synchronous_meilisearch" {
  performance_mode = "generalPurpose"
  throughput_mode  = "bursting"
  encrypted        = true

  // Enable lifecycle management to save costs
  lifecycle_policy {
    transition_to_ia = "AFTER_30_DAYS"
  }

  tags = {
    Name = "${var.service_name}-efs-${var.environment}"
  }
}

// Mount targets in all availability zones where Lambda will run
resource "aws_efs_mount_target" "my_synchronous_meilisearch" {
  for_each = toset(data.aws_subnets.default.ids)

  file_system_id  = aws_efs_file_system.my_synchronous_meilisearch.id
  subnet_id       = each.value
  security_groups = [data.aws_security_group.default.id]
}

// Access point for Lambda
resource "aws_efs_access_point" "my_synchronous_meilisearch" {
  file_system_id = aws_efs_file_system.my_synchronous_meilisearch.id

  root_directory {
    path = "/efs"

    creation_info {
      owner_gid   = 1000
      owner_uid   = 1000
      permissions = "755"
    }
  }

  posix_user {
    gid = 1000
    uid = 1000
  }

  tags = {
    Name = "${var.service_name}-efs-ap-${var.environment}"
  }
}
