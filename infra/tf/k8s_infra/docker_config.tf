module "secrets" {
    source = "../modules/secrets"

    keys = [
        "docker/registry/ghcr.io/username",
        "docker/registry/ghcr.io/password",
    ]
}

locals {
	ghcr_registry_data = {
		".dockerconfigjson" = jsonencode({
			"auths" = {
				"ghcr.io" = {
					"auth" = base64encode("${module.secrets.values["docker/registry/ghcr.io/username"]}:${module.secrets.values["docker/registry/ghcr.io/password"]}")
				}
			}
		})
	}
}
