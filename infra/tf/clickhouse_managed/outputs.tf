output "host" {
	value = clickhouse_service.main.endpoints[0].host
}

output "port_https" {
	value = [for x in clickhouse_service.main.endpoints: x.port if x.protocol == "https"][0]
}

output "port_native_secure" {
	value = [for x in clickhouse_service.main.endpoints: x.port if x.protocol == "nativesecure"][0]
}

