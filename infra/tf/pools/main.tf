terraform {
	required_providers {
		linode = {
			source = "linode/linode"
			version = "1.29.2"
		}
	}
}

module "secrets" {
	source = "../modules/secrets"

	keys = [
		"linode/token",
		"ssh/server/private_key_openssh",
	]
}

