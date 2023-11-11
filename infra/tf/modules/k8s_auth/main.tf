module "docker_secrets" {
	count = var.authenticate_all_docker_hub_pulls ? 1 : 0
	source = "../secrets"

	keys = [
		"docker/registry/docker.io/read/username",
		"docker/registry/docker.io/read/password",
	]
}

module "docker_ghcr_secrets" {
	count = var.deploy_method_cluster ? 1 : 0
	source = "../secrets"

	keys = [
        "docker/registry/ghcr.io/read/username",
        "docker/registry/ghcr.io/read/password",
	]
}

# Create Docker auth secret in every namespace it's used in
resource "kubernetes_secret" "docker_auth" {
	for_each = toset(var.namespaces)

	metadata {
		name = "docker-auth"
		namespace = each.value
	}

	type = "kubernetes.io/dockerconfigjson"

	data = {
		".dockerconfigjson" = jsonencode({
			auths = {
				"https://index.docker.io/v1/" = (
					var.authenticate_all_docker_hub_pulls ?
					{
						auth = base64encode(
							"${module.docker_secrets.values["docker/registry/docker.io/read/username"]}:${module.docker_secrets.values["docker/registry/docker.io/read/password"]}"
						)
					}
					: null
				)
				"ghcr.io" = (
					var.deploy_method_cluster ?
					{
						"auth" = base64encode("${module.docker_ghcr_secrets[0].values["docker/registry/ghcr.io/read/username"]}:${module.docker_ghcr_secrets[0].values["docker/registry/ghcr.io/read/password"]}")
					}
					: null
				)
			}
		})
	}
}
