variable "name_prefix" {
  type = string
}

variable "vpc_id" {
  type = string
}

variable "private_subnet_ids" {
  type = list(string)
}

variable "k8s_version" {
  type = string
}

variable "environment" {
  type = string
}
