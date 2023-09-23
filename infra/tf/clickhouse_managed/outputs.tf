output "host" {
	value = clickhouse_service.main.endpoints[0].host
}

output "port" {
	value = clickhouse_service.main.endpoints[0].port
}
