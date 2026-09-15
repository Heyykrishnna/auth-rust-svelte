# Infrastructure as Code — Terraform

Production-ready Terraform configurations to provision foundational cloud infrastructure for the `secure-auth-platform`.

## Architecture Provisioned

- **Networking**: Multi-AZ VPC with public, private, and database isolated subnets, internet gateway, and NAT gateways.
- **Kubernetes**: AWS EKS cluster with managed auto-scaling node groups and IRSA.
- **Database**: PostgreSQL 16 RDS instance with automated snapshots, encryption at rest, and multi-AZ support in production.
- **Cache**: ElastiCache Redis replication group for token family tracking, rate limiting, and session caching.

## Directory Structure

```text
infrastructure/terraform/
├── main.tf                 # Root module wiring submodules together
├── variables.tf            # Configurable inputs (environment, CIDR, instance sizes)
├── outputs.tf              # Cluster endpoints, database host, Redis address
├── versions.tf             # Provider pins (AWS, Kubernetes, Helm)
├── terraform.tfvars.example# Example variables file
└── modules/
    ├── networking/         # VPC, subnets, route tables, NAT
    ├── kubernetes/         # EKS cluster, node groups, IAM
    ├── database/           # PostgreSQL RDS, subnet group, security group
    └── redis/              # ElastiCache Redis cluster
```

## Quick Start

1. Initialize Terraform:
   ```bash
   terraform init
   ```
2. Create an execution plan:
   ```bash
   terraform plan -var-file=terraform.tfvars.example -out=tfplan
   ```
3. Apply the plan:
   ```bash
   terraform apply tfplan
   ```
