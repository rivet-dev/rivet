output "host" {
	value = "clickhouse.clickhouse.svc.cluster.local"
}

output "port" {
	value = 8123
}

output "ca_crt" {
	value = data.kubernetes_secret.clickhouse_ca.data["ca.crt"]
	sensitive = true
}
