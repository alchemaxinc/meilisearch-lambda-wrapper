// EFS for Lambda - cheapest configuration
// Uses regional (Standard) storage class, General Purpose performance mode,
// and Bursting throughput mode. (Not "One Zone": that storage class requires
// setting `availability_zone_name` on the file system, which isn't set here.)

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

// One mount target per Availability Zone (never per subnet: EFS rejects a
// second mount target in the same AZ). See locals.tf's one_subnet_id_per_az.
// Keyed by subnet id (not AZ) so that, on upgrade from an all-subnets
// for_each, surviving subnets keep the same resource address instead of
// Terraform planning to destroy and recreate every mount target.
resource "aws_efs_mount_target" "my_synchronous_meilisearch" {
  for_each = toset(values(local.one_subnet_id_per_az))

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
