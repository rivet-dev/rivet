resource "kubernetes_namespace" "traefik" {
	metadata {
		name = "traefik"
	}
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

		service = {
		}

		logs = {
			general = {
				level = "DEBUG"
			}
			# TODO: Disable on prod
			access = {
				enabled = true
			}
		}
	})]
}

