resource "kubernetes_namespace" "traefik" {
	metadata {
		name = "traefik"
	}
}

module "traefik_secrets" {
	source = "../modules/secrets"

	keys = [
		"rivet/api_route/token",
	]
}

locals {
	service_traefik = lookup(var.services, "traefik", {
		count = 1
		resources = {
			cpu = 50
			cpu_cores = 0
			memory = 200
		}
	})
}

resource "helm_release" "traefik" {
	name = "traefik"
	namespace = kubernetes_namespace.traefik.metadata.0.name

	repository = "https://traefik.github.io/charts"
	chart = "traefik"
	version = "24.0.0"
	values = [yamlencode({
		# Allows referencing services outside of the traefik namespace
		providers = {
			kubernetesCRD = {
				allowCrossNamespace = true
				labelSelector = "traefik-instance=main"
			}
		}

		commonLabels = {
			"traefik-instance" = "main"
		}

		resources = var.limit_resources ? {
			limits = {
				memory = "${local.service_traefik.resources.memory}Mi"
				cpu = (
					local.service_traefik.resources.cpu_cores > 0 ?
					"${local.service_traefik.resources.cpu_cores * 1000}m"
					: "${local.service_traefik.resources.cpu}m"
				)
			}
		} : null

		additionalArguments = [
			"--providers.http.endpoint=http://rivet-api-route.rivet-service.svc.cluster.local/traefik/config/core?token=${module.traefik_secrets.values["rivet/api_route/token"]}",
			"--providers.http.pollInterval=2.5s",
			# 60s for the long polling requests to gracefully exit + 30s for padding
			"--entryPoints.web.transport.lifeCycle.graceTimeOut=90s",
			"--entryPoints.websecure.transport.lifeCycle.graceTimeOut=90s",
		]

		logs = {
			# general = {
			# 	level = "DEBUG"
			# }
			# NOTE: Do not enable on prod
			# access = {
			# 	enabled = true
			# }
		}

		ports = {
			websecure = {
				tls = {
					enabled = true
					options = "ingress-cloudflare"
				}
			}
		}

		tlsOptions = {
			"ingress-cloudflare" = {
				curvePreferences = ["CurveP384"]

				clientAuth = {
					secretNames = ["ingress-tls-cloudflare-ca-cert"]
					clientAuthType = "RequireAndVerifyClientCert"
				}
			}
		}

		metrics = {
			prometheus = {
				addEntryPointsLabels = true
				addRoutersLabels = true
				addServicesLabels = true
				# See lib/chirp/metrics/src/buckets.rs
				buckets = "0.001,0.0025,0.005,0.01,0.025,0.05,0.1,0.25,0.5,1.0,2.5,5.0,10.0,25.0,50.0,100.0"
			}
		}
	})]
}

data "kubernetes_service" "traefik" {
	depends_on = [helm_release.traefik]

	metadata {
		name = "traefik"
		namespace = kubernetes_namespace.traefik.metadata.0.name
	}
}

