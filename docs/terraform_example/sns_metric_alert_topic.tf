resource "aws_sns_topic" "cloudwatch_metric_alerts" {
  name = "${var.service_name}_${var.environment}_Default_CloudWatch_Alarms_Topic"
}

resource "aws_sns_topic_subscription" "hello_synago_io" {
  topic_arn = aws_sns_topic.cloudwatch_metric_alerts.arn
  protocol  = "email"
  endpoint  = "<your-email-for-alerts>"
}
