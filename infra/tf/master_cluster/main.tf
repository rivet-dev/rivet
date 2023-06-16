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
		"linode/terraform/token",
        "ssh/nebula_lighthouse/private_key_openssh",
        "ssh/salt_master/private_key_openssh",
    ]
}
