output "host" {
	value = cockroach_cluster.main.regions[0].sql_dns
}

output "port" {
	value = 26257
}

output "cluster_ca_crt" {
	value = data.cockroach_cluster_cert.main.cert
}

output "username" {
	value = cockroach_sql_user.root.name
}

output "password" {
	value = random_password.root_password.result
	sensitive = true
}

