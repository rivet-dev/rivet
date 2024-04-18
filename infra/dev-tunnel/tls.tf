resource "tls_private_key" "ssh_key" {
	algorithm = "RSA"
	rsa_bits = 2048
}

resource "local_file" "ssh_key_file" {
	filename = "/tmp/tunnel_id_rsa"
	content  = tls_private_key.ssh_key.private_key_pem
	file_permission = "0600"
}
