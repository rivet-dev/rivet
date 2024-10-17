output "host" {
	value = {
		for k, v in local.redis_svcs:
		k => aiven_redis.main[k].service_host
	}
}

output "port" {
	value = {
		for k, v in local.redis_svcs:
		k => aiven_redis.main[k].service_port
	}
}

