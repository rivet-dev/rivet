terraform {
	required_providers {
		b2 = {
			source = "Backblaze/b2"
			version = "0.8.1"
		}
	}
}

module "secrets" {
    source = "../modules/secrets"

    keys = [
        "b2/terraform/key_id",
        "b2/terraform/key",
    ]
}

provider "b2" {
	application_key_id = module.secrets.values["b2/terraform/key_id"]
	application_key = module.secrets.values["b2/terraform/key"]
}

data "b2_account_info" "main" {}
