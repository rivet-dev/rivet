output "host" {
	value = "cockroachdb.cockroachdb.svc.cluster.local"
}

output "port" {
	value = 26257
}

output "cluster_ca_crt" {
	value = data.kubernetes_config_map.root_ca.data["ca.crt"]
}

output "username" {
	value = "rivet-root"
}

output "password" {
	value = random_password.root_password.result
	sensitive = true
}

