module "secrets" {
    source = "../modules/secrets"

    keys = [
        "docker/registry/ghcr.io/read/username",
        "docker/registry/ghcr.io/read/password",
    ]
}

locals {
	ghcr_registry_data = {
		".dockerconfigjson" = jsonencode({
			"auths" = {
				"ghcr.io" = {
					"auth" = base64encode("${module.secrets.values["docker/registry/ghcr.io/read/username"]}:${module.secrets.values["docker/registry/ghcr.io/read/password"]}")
				}
			}
		})
	}
}
