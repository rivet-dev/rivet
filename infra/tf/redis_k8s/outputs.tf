locals {
	redis_svcs = {
		"persistent" = {}
		"ephemeral" = {}
	}
}

output "host" {
	# Use headless endpoint since it goes directly to the pods
	value = {
		for k, _ in local.redis_svcs:
		k => "redis.redis-${k}.svc.cluster.local"
	}
}

output "port" {
	value = {
		for k, _ in local.redis_svcs:
		k => 6379
	}
}
