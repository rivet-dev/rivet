locals {
	service_nsfw_api = lookup(var.services, "nsfw-api", {
		count = 1
		resources = {
			cpu = 250
			cpu_cores = 0
			memory = 512
		}
	})
}

resource "kubernetes_namespace" "nsfw_api" {
	metadata {
		name = "nsfw-api"
	}
}

resource "kubernetes_priority_class" "nsfw_api_priority" {
	metadata {
		name = "nsfw-api-priority"
	}

	value = 40
}

resource "kubernetes_deployment" "nsfw_api" {
	depends_on = [module.docker_auth]

	metadata {
		name = "nsfw-api"
		namespace = kubernetes_namespace.nsfw_api.metadata[0].name
	}

	spec {
		replicas = local.service_nsfw_api.count

		selector {
			match_labels = {
				"app.kubernetes.io/name" = "nsfw-api"
			}
		}

		template {
			metadata {
				labels = {
					"app.kubernetes.io/name" = "nsfw-api"
				}
			}

			spec {
				priority_class_name = "nsfw-api-priority"
				
				# MARK: Docker auth
				image_pull_secrets {
					name = "docker-auth"
				}

				container {
					image = "eugencepoi/nsfw_api@sha256:087d880e38b82e5cbee761bafd50e5093a40f813d3f0e77a8077f661cbcdb414"
					name = "nsfw-api"

					env {
					  name = "PORT"
					  value = 21900
					}

					port {
						name = "http"
						container_port = 21900
					}
					
					dynamic "resources" {
						for_each = var.limit_resources ? [0] : []

						content {
							limits = {
								memory = "${local.service_nsfw_api.resources.memory}Mi"
								cpu = (
									local.service_nsfw_api.resources.cpu_cores > 0 ?
									"${local.service_nsfw_api.resources.cpu_cores * 1000}m"
									: "${local.service_nsfw_api.resources.cpu}m"
								)
							}
						}
					}
				}
			}
		}
	}
}

resource "kubernetes_service" "nsfw_api" {
	metadata {
		name = "nsfw-api"
		namespace = kubernetes_namespace.nsfw_api.metadata[0].name
	}
	spec {
		selector = {
			"app.kubernetes.io/name" = kubernetes_deployment.nsfw_api.metadata.0.name
		}

		port {
			protocol = "TCP"
			port = 21900
			target_port = "http"
		}
	}
}
