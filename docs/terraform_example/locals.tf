locals {
  efs_mount_path = "/mnt/efs"
  bootstrap_tag  = "bootstrap"

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
