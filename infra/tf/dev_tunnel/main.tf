terraform {
	required_providers {
		linode = {
			source = "linode/linode"
			version = "~> 1.23.0"
		}
        docker = {
            source  = "kreuzwerker/docker"
            version = "~> 2.15.0"
        }
	}
}

module "secrets" {
	source = "../modules/secrets"

	keys = [
		"linode/token",
	]
}

