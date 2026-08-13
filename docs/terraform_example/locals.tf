locals {
  efs_mount_path = "/mnt/efs"
  bootstrap_tag  = "bootstrap"

  # One subnet id per Availability Zone, keyed by AZ. Iterating subnets in a
  # stable (sorted) order and using the `...` grouping operator keeps this a
  # deterministic map: the first subnet encountered for each AZ wins. EFS
  # rejects a second mount target in the same AZ, so this guarantees
  # `aws_efs_mount_target.my_synchronous_meilisearch` never attempts that,
  # even if the default VPC is customized to have more than one subnet per AZ.
  subnet_id_by_az = {
    for subnet_id in sort(data.aws_subnets.default.ids) :
    data.aws_subnet.default[subnet_id].availability_zone => subnet_id...
  }
  one_subnet_id_per_az = { for az, ids in local.subnet_id_by_az : az => ids[0] }

  cloudwatch_metric_error_filters = [
    {
      name    = "panic"
      pattern = "panic"
    },
    # {
    #    "lvl": "ERROR",
    # ...
    {
      name    = "Error"
      pattern = "ERROR"
    },
    # msg": "failed to handle message: foobar3, not ACKing, aborting, error: validation of request failed",
    #    "stacktrace": "main.
    # ...
    {
      name    = "Traceback"
      pattern = "stacktrace"
    },
  ]
}
