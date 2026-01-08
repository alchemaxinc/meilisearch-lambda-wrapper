resource "aws_cloudwatch_log_group" "my_synchronous_meilisearch_api" {
  name              = "/aws/lambda/${aws_lambda_function.my_synchronous_meilisearch_api.function_name}"
  retention_in_days = 14
}

resource "aws_cloudwatch_log_metric_filter" "my_synchronous_meilisearch_api" {
  for_each = { for filter in local.cloudwatch_metric_error_filters : filter.name => filter }

  name           = each.value.name
  log_group_name = aws_cloudwatch_log_group.my_synchronous_meilisearch_api.name
  pattern        = each.value.pattern

  metric_transformation {
    name      = "${aws_lambda_function.my_synchronous_meilisearch_api.function_name}_Errors"
    namespace = "LogMetrics"
    value     = "1"
  }
}
resource "aws_cloudwatch_metric_alarm" "my_synchronous_meilisearch_api" {
  alarm_name          = "${aws_lambda_function.my_synchronous_meilisearch_api.function_name}_Alarm"
  comparison_operator = "GreaterThanOrEqualToThreshold"
  evaluation_periods  = 1
  metric_name         = "${aws_lambda_function.my_synchronous_meilisearch_api.function_name}_Errors"
  namespace           = "LogMetrics"
  period              = 3600 # 1 hour
  statistic           = "Sum"
  threshold           = 1
  alarm_description   = "This metric monitors the Lambda function logs for errors"
  treat_missing_data  = "notBreaching"
  alarm_actions       = [aws_sns_topic.cloudwatch_metric_alerts.arn]
}
