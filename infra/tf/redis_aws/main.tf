locals {
	names = {
		for k, _ in var.redis_dbs:
		k => "rivet-${var.namespace}-${k}"
	}

	cluster_count = 2
	shard_count = 2

	persistent_dbs = {
		for k, v in var.redis_dbs:
		k => {
			username = module.secrets.values["redis/${local.names[k]}/username"]
			password = module.secrets.values["redis/${local.names[k]}/password"]
		}
		if v.persistent
	}
	nonpersistent_dbs = {
		for k, v in var.redis_dbs:
		k => {
			username = module.secrets.values["redis/${local.names[k]}/username"]
			password = module.secrets.values["redis/${local.names[k]}/password"]
		}
		if !v.persistent
	}
}

module "secrets" {
	source = "../modules/secrets"

	keys = flatten([
		for n in local.names:
		[
			"redis/${n}/username",
			"redis/${n}/password",
		]
	])
}

# MARK: Non-persistent
resource "aws_elasticache_replication_group" "main" {
	for_each = local.nonpersistent_dbs

	automatic_failover_enabled  = local.cluster_count > 1
	# AZ count must match the cluster count
	preferred_cache_cluster_azs = slice(data.terraform_remote_state.k8s_cluster_aws.outputs.azs, 0, local.cluster_count)
	replication_group_id = local.names[each.key]
	description = local.names[each.key]
	node_type = "cache.t4g.micro"
	num_cache_clusters = local.cluster_count
	at_rest_encryption_enabled = true
	transit_encryption_enabled = true
	engine_version = "7.0"
	parameter_group_name = "default.redis7"
	subnet_group_name = aws_elasticache_subnet_group.main.name
	user_group_ids = [aws_elasticache_user_group.main[each.key].id]

	lifecycle {
		ignore_changes = [num_cache_clusters]
	}
}

resource "aws_elasticache_cluster" "main" {
	for_each = merge([
		for k, v in local.nonpersistent_dbs:
		{
			for i in range(local.cluster_count):
			"${k}-${i}" => merge(v, {
				db = k
			})
		}
	]...)

	cluster_id = each.key
	replication_group_id = aws_elasticache_replication_group.main[each.value.db].id
	transit_encryption_enabled = false
}

resource "aws_elasticache_subnet_group" "main" {
	name = "rivet-${var.namespace}"
	subnet_ids = data.terraform_remote_state.k8s_cluster_aws.outputs.private_subnets
}

# Remove all capabilities of default user
resource "aws_elasticache_user" "default" {
	for_each = local.nonpersistent_dbs

	user_id = "${local.names[each.key]}-default"
	user_name = "default"
	access_string = "off -@all"
	engine = "REDIS"
	no_password_required = true
}

resource "aws_elasticache_user" "root" {
	for_each = local.nonpersistent_dbs

	user_id = each.value.username
	user_name = each.value.username
	access_string = "on ~* &* +@all"
	engine = "REDIS"
	
	passwords = [each.value.password]
}

resource "aws_elasticache_user_group" "main" {
	for_each = local.nonpersistent_dbs

	engine = "REDIS"
	user_group_id = local.names[each.key]
	user_ids = [
		aws_elasticache_user.default[each.key].user_id,
		aws_elasticache_user.root[each.key].user_id,
	]
}


# MARK: Persistent
data "aws_subnet" "private_subnets" {
	for_each = toset(data.terraform_remote_state.k8s_cluster_aws.outputs.private_subnets)

	id = each.value
}

resource "aws_memorydb_cluster" "main" {
	for_each = local.persistent_dbs

	name = local.names[each.key]
	node_type = "db.t4g.small"
	num_shards = local.shard_count
	acl_name = aws_memorydb_acl.main[each.key].name
	engine_version = "7.0"
	parameter_group_name = "default.memorydb-redis7"
	snapshot_retention_limit = 7
	subnet_group_name = aws_memorydb_subnet_group.main.id
	security_group_ids = [data.terraform_remote_state.k8s_cluster_aws.outputs.eks_cluster_security_group_id]

	# acl_name = "open-access"
	# tls_enabled = false
}

resource "aws_memorydb_subnet_group" "main" {
	name = "rivet-${var.namespace}"

	# HACK: us-east-1c not supported on MemoryDB
	subnet_ids = [
		for x in data.aws_subnet.private_subnets:
		x.id
		if x.availability_zone != "us-east-1c"
	]
}

resource "aws_memorydb_user" "root" {
	for_each = local.persistent_dbs

	user_name = each.value.username
	access_string = "on ~* &* +@all"

	authentication_mode {
		type = "password"
		passwords = [each.value.password]
	}
}

resource "aws_memorydb_acl" "main" {
	for_each = local.persistent_dbs

	name = local.names[each.key]
	user_names = [aws_memorydb_user.root[each.key].user_name]
}
