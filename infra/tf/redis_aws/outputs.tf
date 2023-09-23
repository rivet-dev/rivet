output "host" {
	value = merge(
		{
			for k, v in var.redis_dbs:
			k => split(":", aws_elasticache_replication_group.main[k].primary_endpoint_address)[0]
			if !v.persistent
		},
		{
			for k, v in var.redis_dbs:
			k => aws_memorydb_cluster.main[k].cluster_endpoint[0].address
			if v.persistent
		}
	)
}

output "port" {
	value = merge(
		{
			for k, v in var.redis_dbs:
			k => 6379
			if !v.persistent
		},
		{
			for k, v in var.redis_dbs:
			k => aws_memorydb_cluster.main[k].cluster_endpoint[0].port
			if v.persistent
		}
	)
}
