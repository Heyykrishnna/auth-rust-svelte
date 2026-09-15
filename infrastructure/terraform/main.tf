provider "aws" {
  region = var.aws_region

  default_tags {
    tags = {
      Project     = var.project_name
      Environment = var.environment
      ManagedBy   = "Terraform"
    }
  }
}

locals {
  name_prefix = "${var.project_name}-${var.environment}"
}

module "networking" {
  source = "./modules/networking"

  name_prefix        = local.name_prefix
  vpc_cidr           = var.vpc_cidr
  availability_zones = var.availability_zones
  environment        = var.environment
}

module "kubernetes" {
  source = "./modules/kubernetes"

  name_prefix        = local.name_prefix
  vpc_id             = module.networking.vpc_id
  private_subnet_ids = module.networking.private_subnet_ids
  k8s_version        = var.k8s_version
  environment        = var.environment
}

module "database" {
  source = "./modules/database"

  name_prefix             = local.name_prefix
  vpc_id                  = module.networking.vpc_id
  database_subnet_ids     = module.networking.database_subnet_ids
  allowed_security_groups = [module.kubernetes.cluster_security_group_id]
  instance_class          = var.db_instance_class
  allocated_storage       = var.db_allocated_storage
  environment             = var.environment
}

module "redis" {
  source = "./modules/redis"

  name_prefix             = local.name_prefix
  vpc_id                  = module.networking.vpc_id
  cache_subnet_ids        = module.networking.database_subnet_ids
  allowed_security_groups = [module.kubernetes.cluster_security_group_id]
  node_type               = var.redis_node_type
  num_cache_clusters      = var.redis_num_cache_clusters
  environment             = var.environment
}
