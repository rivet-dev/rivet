terraform {
	required_providers {
		aiven = {
			source = "aiven/aiven"
			version = "4.12.1"
		}
	}
}

provider "aiven" {
	api_token = module.secrets.values["aiven/api_token"]
}

