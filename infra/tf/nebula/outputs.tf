output "nebula_ca_cert" {
	value = nebula_ca.main.cert
}

output "nebula_ca_key" {
	value = nebula_ca.main.key
    sensitive = true
}
