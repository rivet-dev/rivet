resource "null_resource" "update_sshd_config" {
	depends_on = [linode_instance.tunnel]
    triggers = {
      override = 2
    }

	connection {
		type = "ssh"
		user = "root"
		private_key = tls_private_key.ssh_key.private_key_pem
		host = linode_instance.tunnel.ip_address
	}

	provisioner "local-exec" {
		command = <<-EOT
		 	# Wait for SSH
			while ! nc -z ${linode_instance.tunnel.ip_address} 22; do
				echo "Waiting for SSH to be available..."
				sleep 2
			done

			# Update config
			ssh -o StrictHostKeyChecking=no -i ${local_file.ssh_key_file.filename} root@${linode_instance.tunnel.ip_address} \
            "echo 'GatewayPorts yes' > /etc/ssh/sshd_config.d/dev_tunnel.conf && \
            systemctl restart ssh"
		EOT
	}
}

resource "docker_container" "ssh_tunnel" {
    depends_on = [ null_resource.update_sshd_config]

	image = "debian:11"
	name = "rivet-tunnel"
	restart = "unless-stopped"
	network_mode = "host"
	command = [
		"sh",
		"-c",
		# StrictHostKeyChecking=no = disables prompting before adding remote host to hosts file
		# -v = verbose
		# -N = don't execute command
		# -T = no TTY
		# -R = reverse proxy
		<<EOF
		apt-get update -y
		apt-get install -y openssh-client 
		while true; do
			echo 'Connecting...'
			ssh -o StrictHostKeyChecking=no -i /root/.ssh/id_rsa -vNT -R 0.0.0.0:80:127.0.0.1:80 -R 0.0.0.0:443:127.0.0.1:443 -R 0.0.0.0:5000:127.0.0.1:5000 -R 0.0.0.0:9000:127.0.0.1:9000 root@${linode_instance.tunnel.ip_address}
			sleep 5
		done
		EOF
	]
	volumes {
		host_path = local_file.ssh_key_file.filename
		container_path = "/root/.ssh/id_rsa"
	}
}
