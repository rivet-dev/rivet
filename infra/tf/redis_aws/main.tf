terraform {
	required_providers {
		aws = {
			source = "hashicorp/aws"
			version = "5.16.0"
		}
	}
}

provider "aws" {
	region = "us-east-1"

	default_tags {
		tags = {
			Namespace = var.namespace
		}
	}
}

locals {
	names = {
		for k, _ in var.redis_dbs:
		k => "rivet-${var.namespace}-${k}"
	}

	cluster_count = 2

	# AZ count must match the cluster count
	azs = slice(data.terraform_remote_state.k8s_aws.outputs.azs, 0, local.cluster_count)
}

resource "random_password" "password" {
	for_each = var.redis_dbs

	length = 32
	special = false
}

resource "aws_elasticache_cluster" "main" {
	for_each = var.redis_dbs

	cluster_id = local.names[each.key]
	replication_group_id = aws_elasticache_replication_group.main[each.key].id
}

resource "aws_elasticache_replication_group" "main" {
	for_each = var.redis_dbs

	automatic_failover_enabled  = true
	preferred_cache_cluster_azs = local.azs
	num_cache_clusters = local.cluster_count
	replication_group_id = local.names[each.key]
	description = local.names[each.key]
	node_type = "cache.t4g.micro"
	auth_token = random_password.password[each.key].result
	at_rest_encryption_enabled = true
	transit_encryption_enabled = true
	engine_version = "7.0"
	subnet_group_name = aws_elasticache_subnet_group.main.name
}


resource "aws_elasticache_subnet_group" "main" {
	name = "rivet-${var.namespace}"
	subnet_ids = data.terraform_remote_state.k8s_aws.outputs.private_subnets
}

