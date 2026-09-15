output "endpoint" {
  value = aws_db_instance.postgres.endpoint
}

output "database_name" {
  value = aws_db_instance.postgres.db_name
}

output "database_username" {
  value = aws_db_instance.postgres.username
}

output "database_password_secret_name" {
  value = "${var.name_prefix}-db-password"
}
