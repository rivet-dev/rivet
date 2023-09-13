output "host"{
	value = "clickhouse.clickhouse.svc.cluster.local"
}

output "port"{
	value = 8123
}

output "cluster_ca_crt"{
	value = data.kubernetes_config_map.root_ca.data["ca.crt"]
}

output "username" {
	value = "default"
}

output "password" {
	value = random_password.default.result
	sensitive = true
}

