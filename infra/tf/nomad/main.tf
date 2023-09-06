module "secrets" {
	source = "../modules/secrets"

	keys = flatten([
		var.deploy_method_cluster ? [
			"cloudflare/access/terraform_nomad/client_id",
			"cloudflare/access/terraform_nomad/client_secret",
		] : [],
		var.authenticate_all_docker_hub_pulls ? [
			"docker/registry/docker.io/username",
			"docker/registry/docker.io/password",
		] : [],
	])
}
