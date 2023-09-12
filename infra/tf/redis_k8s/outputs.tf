output "host"{
	value = {
		for k, _ in var.redis_dbs:
		k => "redis-master.redis-${k}.svc.cluster.local"
	}
}

output "port"{
	value = {
		for k, _ in var.redis_dbs:
		k => 6379
	}
}

output "cluster_ca_crt"{
	value = {
		for k, _ in var.redis_dbs:
		k => data.kubernetes_config_map.root_ca[k].data["ca.crt"]
	}
}

output "password" {
	value = {
		for k, _ in var.redis_dbs:
		k => random_password.password[k].result
	}
	sensitive = true
}

