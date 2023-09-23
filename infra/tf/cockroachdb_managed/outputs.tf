output "host" {
	value = cockroach_cluster.main.regions[0].sql_dns
}

output "port" {
	value = 26257
}
