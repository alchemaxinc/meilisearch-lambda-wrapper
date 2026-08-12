resource "aws_sns_topic" "cloudwatch_metric_alerts" {
  name = "${var.service_name}_${var.environment}_Default_CloudWatch_Alarms_Topic"
}

# Only created when var.alert_email is set. AWS validates the "email"
# protocol endpoint's format, so creating this with an unset/placeholder
# address would fail terraform apply; skipping it entirely when no real
# address is configured avoids that instead of shipping an invalid default.
resource "aws_sns_topic_subscription" "alert_email" {
  count = var.alert_email == null ? 0 : 1

  topic_arn = aws_sns_topic.cloudwatch_metric_alerts.arn
  protocol  = "email"
  endpoint  = var.alert_email
}
