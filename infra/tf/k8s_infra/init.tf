resource "kubernetes_namespace" "rivet_service" {
	metadata {
		name = "rivet-service"
	}
}

# NOTE: Must use kubectl_manifest because kubernetes_manifest doesn't work with CRDs. If this stops working
# correctly replace with a raw helm chart: https://artifacthub.io/packages/helm/wikimedia/raw
# https://github.com/hashicorp/terraform-provider-kubernetes/issues/1367#
resource "kubectl_manifest" "ingress_tls" {
	depends_on = [helm_release.traefik, kubernetes_namespace.traefik, kubernetes_namespace.imagor]

	yaml_body = yamlencode({
		apiVersion = "traefik.containo.us/v1alpha1"
		kind = "TLSOption"

		metadata = {
			name = "ingress-tls"
			namespace = kubernetes_namespace.traefik.metadata.0.name
		}

		spec = {
			curvePreferences = [ "CurveP384" ]

			clientAuth = {
				secretNames = [ "ingress-tls-ca-cert" ]
				clientAuthType = "RequireAndVerifyClientCert"
			}
		}
	})
}

# Must be created in every namespace it is used in
resource "kubernetes_secret" "ingress_tls_cert" {
	for_each = toset([
		for x in [kubernetes_namespace.traefik, kubernetes_namespace.imagor]:
		x.metadata.0.name
	])

	metadata {
		name = "ingress-tls-cert"
		namespace = each.value
	}

	type = "kubernetes.io/tls"

	data = {
		"tls.crt" = data.terraform_remote_state.tls.outputs.tls_cert_cloudflare_rivet_gg.cert_pem
		"tls.key" = data.terraform_remote_state.tls.outputs.tls_cert_cloudflare_rivet_gg.key_pem
	}
}

resource "kubernetes_secret" "ingress_tls_ca_cert" {
	metadata {
		name = "ingress-tls-ca-cert"
		namespace = kubernetes_namespace.traefik.metadata.0.name
	}

	data = {
		"tls.ca" = data.terraform_remote_state.tls.outputs.tls_cert_cloudflare_ca
	}
}

resource "kubernetes_config_map" "health_checks" {
	metadata {
		name = "health-checks"
		namespace = kubernetes_namespace.rivet_service.metadata.0.name
	}

	data = {
		"health-checks.sh" = <<-EOF
			#!/bin/sh
			set -uf

			# Install curl
			if ! [ -x "$(command -v curl)" ]; then
				if ! [ -x "$(command -v apk)" ]; then
					apt-get install -y curl
				else
					apk add --no-cache curl
				fi
			fi

			curl 127.0.0.1:${var.k8s_health_port}/health/liveness
			EXIT_STATUS=$?
			if [ $EXIT_STATUS -ne 0 ]; then
				echo "health server liveness check failed"
				exit $EXIT_STATUS
			fi

			curl 127.0.0.1:${var.k8s_health_port}/health/crdb/db-user
			EXIT_STATUS=$?
			if [ $EXIT_STATUS -ne 0 ]; then
				echo "cockroach check failed"
				exit $EXIT_STATUS
			fi

			curl 127.0.0.1:${var.k8s_health_port}/health/nats
			EXIT_STATUS=$?
			if [ $EXIT_STATUS -ne 0 ]; then
				echo "nats connection check failed"
				exit $EXIT_STATUS
			fi

			curl 127.0.0.1:${var.k8s_health_port}/health/redis/redis-chirp
			EXIT_STATUS=$?
			if [ $EXIT_STATUS -ne 0 ]; then
				echo "redis chirp connection check failed"
				exit $EXIT_STATUS
			fi

			# Static endpoint flag
			if [[ "$*" == *"--static"* ]]; then
				curl 127.0.0.1:${var.k8s_health_port}/
				EXIT_STATUS=$?
				if [ $EXIT_STATUS -ne 0 ]; then
					echo "static root accessible check failed"
					exit $EXIT_STATUS
				fi
			fi

			echo Ok
			echo
			EOF
	}
}

module "docker_secrets" {
	source = "../modules/secrets"

	keys = flatten([
		var.authenticate_all_docker_hub_pulls ? [
			"docker/docker_io/username",
			"docker/docker_io/password",
		] : [],
	])
}

module "docker_ghcr_secrets" {
	source = "../modules/secrets"

	keys = flatten([
		"docker/ghcr/token",
	])

	optional = true
}

# NOTE: Needs to be created in every K8s namespace it is used in
resource "kubernetes_secret" "docker_auth" {
	for_each = toset([
		for x in [kubernetes_namespace.redis, kubernetes_namespace.rivet_service]:
		x.metadata.0.name
	])

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
							username = module.docker_secrets.values["docker/docker_io/username"]
							password = module.docker_secrets.values["docker/docker_io/password"]
							auth = base64encode(
								"${module.docker_secrets.values["docker/docker_io/username"]}:${module.docker_secrets.values["docker/docker_io/password"]}"
							)
						}
						: null
				)
				"ghcr.io" = (
					module.docker_ghcr_secrets.values["docker/ghcr/token"] != null ?
					{
						username = "$"
						pasword = module.docker_ghcr_secrets.values["docker/ghcr/token"]
						auth = base64encode(
							"$:${module.docker_ghcr_secrets.values["docker/ghcr/token"]}"
						)
					}
					: null
				)
			}
		})
	}
}
