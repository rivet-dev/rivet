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


resource "helm_release" "traefik" {
	name = "traefik"
	namespace = "traefik"

	repository = "https://traefik.github.io/charts"
	chart = "traefik"
	values = [yamlencode({
		# Allows referencing services outside of the traefik namespace
		providers = {
			kubernetesCRD = {
				allowCrossNamespace = true
			}
		}

		additionalArguments = [
			"--providers.http.endpoint=http://rivet-api-route.rivet-service.svc.cluster.local:80/traefik/config/core?token=${module.traefik_secrets.values["rivet/api_route/token"]}",
			"--providers.http.pollInterval=2.5s",
		]

		logs = {
			general = {
				level = "DEBUG"
			}
			# TODO: Disable on prod
			access = {
				enabled = true
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

