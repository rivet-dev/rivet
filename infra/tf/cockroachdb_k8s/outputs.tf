output "host" {
	value = "cockroachdb.cockroachdb.svc.cluster.local"
}

output "port" {
	value = 26257
}

output "ca_crt" {
	value = data.kubernetes_secret.crdb_ca.data["ca.crt"]
	sensitive = true
}
