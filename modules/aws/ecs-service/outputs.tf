# A real, reachable URL — the output the predecessor never emitted, forcing consumers
# to reconstruct the ALB DNS by hand. https when a cert is present.
output "url" {
  value = local.enable_tls ? "https://${aws_lb.this.dns_name}" : "http://${aws_lb.this.dns_name}"
}

output "alb_dns" {
  value = aws_lb.this.dns_name
}

output "service_arn" {
  value = aws_ecs_service.this.id
}

output "task_role_arn" {
  value = aws_iam_role.task.arn
}

output "execution_role_arn" {
  value = local.exec_role
}

output "log_group" {
  value = aws_cloudwatch_log_group.this.name
}

output "cluster" {
  value = aws_ecs_cluster.this.name
}
