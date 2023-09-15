output "host" {
	value = {
		for k, _ in var.redis_dbs:
		k => "redis-master.redis-${k}.svc.cluster.local"
	}
}

output "port" {
	value = {
		for k, _ in var.redis_dbs:
		k => 6379
	}
}

output "ca_crt" {
	value = {
		for k, _ in var.redis_dbs:
		k => data.kubernetes_secret.redis_ca[k].data["ca.crt"]
	}
	sensitive = true
}
