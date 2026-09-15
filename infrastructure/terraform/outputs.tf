output "vpc_id" {
  description = "ID of the Provisioned VPC"
  value       = module.networking.vpc_id
}

output "eks_cluster_name" {
  description = "Name of the EKS Cluster"
  value       = module.kubernetes.cluster_name
}

output "eks_cluster_endpoint" {
  description = "Endpoint of the EKS API server"
  value       = module.kubernetes.cluster_endpoint
}

output "database_endpoint" {
  description = "Connection endpoint for PostgreSQL RDS"
  value       = module.database.endpoint
}

output "redis_primary_endpoint" {
  description = "Primary endpoint address for ElastiCache Redis"
  value       = module.redis.primary_endpoint
}
