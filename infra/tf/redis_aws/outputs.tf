output "host"{
	value = {
		for k, _ in var.redis_dbs:
		k => split(":", aws_elasticache_replication_group.main[k].primary_endpoint_address)[0]
	}
}

output "port"{
	value = {
		for k, _ in var.redis_dbs:
		k => tonumber(split(":", aws_elasticache_replication_group.main[k].primary_endpoint_address)[1])
	}
}

output "cluster_ca_crt"{
	value = {
		for k, _ in var.redis_dbs:
		k => null
	}
}

output "password" {
	value = {
		for k, _ in var.redis_dbs:
		k => random_password.password[k].result
	}
	sensitive = true
}

