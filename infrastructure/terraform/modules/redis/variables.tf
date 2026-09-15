variable "name_prefix" {
  type = string
}

variable "vpc_id" {
  type = string
}

variable "cache_subnet_ids" {
  type = list(string)
}

variable "allowed_security_groups" {
  type = list(string)
}

variable "node_type" {
  type = string
}

variable "num_cache_clusters" {
  type = number
}

variable "environment" {
  type = string
}
