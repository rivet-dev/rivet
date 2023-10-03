output "host" {
	# Use headless endpoint since it goes directly to the pods
	value = {
		for k, _ in var.redis_dbs:
		k => "redis-redis-cluster-headless.redis-${k}.svc.cluster.local"
	}
}

output "port" {
	value = {
		for k, _ in var.redis_dbs:
		k => 6379
	}
}
