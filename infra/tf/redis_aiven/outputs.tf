output "host" {
	value = {
		for k, v in var.redis_dbs:
		k => aiven_redis.main[k].service_host
	}
}

output "port" {
	value = {
		for k, v in var.redis_dbs:
		k => aiven_redis.main[k].service_port
	}
}

