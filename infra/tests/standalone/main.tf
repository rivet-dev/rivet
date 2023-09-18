terraform {
	required_providers {
		linode = {
			source  = "linode/linode"
		}
	}
}

provider "linode" {
	token = var.linode_token
}

module "server" {
    source = "../../tf/modules/generic_server"

	namespace = var.namespace
	private_key_openssh = var.private_key_openssh

	region = {
		provider = "linode"
		provider_region = "us-southeast"
		netnum = 0
	}

	size = "g6-standard-4"
	label = "master-${var.namespace}"
	tags = [var.namespace, "master"]

	# TODO: Filter inbound from Cloudflare
	firewall_inbound = [
		{
			label = "http-tcp"
			ports = "80"
			protocol = "tcp"
			inbound_ipv4_cidr = ["0.0.0.0/0"]
			inbound_ipv6_cidr = ["::/0"]
		},
		{
			label = "https-tcp"
			ports = "443"
			protocol = "tcp"
			inbound_ipv4_cidr = ["0.0.0.0/0"]
			inbound_ipv6_cidr = ["::/0"]
		},
	]
}

resource "null_resource" "install_nix" {
	depends_on = [module.server]

	connection {
		type = "ssh"
		host = module.server.host
		user = module.server.user
		private_key = module.server.private_key_openssh
	}

	provisioner "remote-exec" {
		inline = [
			"#!/bin/bash",
			"set -euf -o pipefail",
			"curl -L https://nixos.org/nix/install | sh -s -- --daemon --yes",
			"source /root/.nix-profile/etc/profile.d/nix.sh",
		]
	}
}

resource "null_resource" "clone_repo" {
	depends_on = [null_resource.install_nix]

	connection {
		type = "ssh"
		host = module.server.host
		user = module.server.user
		private_key = module.server.private_key_openssh
	}

	provisioner "remote-exec" {
		inline = [
			"#!/bin/bash",
			"set -euf -o pipefail",
			"source /root/.nix-profile/etc/profile.d/nix.sh",
			"echo 'Installing Git'",
			"nix-env -i git git-lfs",
			"echo 'Cloning repo'",
			"git clone 'https://${nonsensitive(var.github_pat)}@github.com/rivet-gg/backend'",
			"echo 'Checking out ref'",
			"cd backend",
			"git checkout '${var.repo_ref}'",
		]
	}
}

resource "null_resource" "repo_setup" {
	depends_on = [null_resource.clone_repo]

	connection {
		type = "ssh"
		host = module.server.host
		user = module.server.user
		private_key = module.server.private_key_openssh
	}

	provisioner "remote-exec" {
		inline = [
			"#!/bin/bash",
			"set -euf -o pipefail",
			"source /root/.nix-profile/etc/profile.d/nix.sh",
			"cd backend",
			"nix-shell --run './scripts/setup.sh'",
		]
	}
}

resource "null_resource" "upload_configs" {
	depends_on = [null_resource.repo_setup]

	connection {
		type = "ssh"
		host = module.server.host
		user = module.server.user
		private_key = module.server.private_key_openssh
	}

	provisioner "file" {
		content = var.namespace_config
		destination = "/root/backend/namespaces/${var.namespace}.toml"
	}

	provisioner "file" {
		content = var.namespace_secrets
		destination = "/root/backend/secrets/${var.namespace}.toml"
	}
}

resource "null_resource" "bolt_init" {
	depends_on = [null_resource.upload_configs]

	connection {
		type = "ssh"
		host = module.server.host
		user = module.server.user
		private_key = module.server.private_key_openssh
	}

	provisioner "remote-exec" {
		inline = [
			"#!/bin/bash",
			"set -euf -o pipefail",
			"source /root/.nix-profile/etc/profile.d/nix.sh",
			"cd backend",
			"nix-shell --run 'bolt init --yes ${var.namespace}'",
		]
	}
}

resource "null_resource" "bolt_test" {
	depends_on = [null_resource.bolt_init]

	connection {
		type = "ssh"
		host = module.server.host
		user = module.server.user
		private_key = module.server.private_key_openssh
	}

	provisioner "remote-exec" {
		inline = [
			"#!/bin/bash",
			"set -euf -o pipefail",
			"source /root/.nix-profile/etc/profile.d/nix.sh",
			"cd backend",
			"nix-shell --run 'bolt test -t user-*'",
		]
	}
}
