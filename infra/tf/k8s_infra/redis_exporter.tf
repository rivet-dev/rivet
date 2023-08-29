# This gets ran inside of K8s instead of inside the Redis node since we use a
# managed Redis service for production environments.

locals {
	service_redis_exporter = lookup(var.services, "redis-exporter", {
		count = 1
		resources = {
			cpu = 100
			cpu_cores = 0
			memory = 256
		}
	})
}

module "redis_secrets" {
	source = "../modules/secrets"

	keys = flatten([
		for k, v in var.redis_svcs:
		[
			"redis/${k}/username",
			"redis/${k}/password",
		]
	])
	optional = true
}

resource "kubernetes_priority_class" "redis_exporter_priority" {
	metadata {
		name = "redis-exporter-priority"
	}

	value = 40
}

resource "kubernetes_deployment" "redis_exporter" {
	depends_on = [kubernetes_namespace.redis, kubernetes_secret.docker_auth]

	for_each = var.redis_svcs
	
	metadata {
		name = "redis-exporter-${each.key}"
		namespace = "redis"
	}

	spec {
		replicas = local.service_redis_exporter.count
		
		selector {
			match_labels = {
				"app.kubernetes.io/name" = "redis-exporter-${each.key}"
			}
		}

		template {
			metadata {
				labels = {
					"app.kubernetes.io/name" = "redis-exporter-${each.key}"
				}
			}

			spec {
				priority_class_name = "redis-exporter-priority"
				
				dynamic "image_pull_secrets" {
					for_each = var.authenticate_all_docker_hub_pulls ? toset([1]) : toset([])

					content {
						name = "docker-auth"
					}
				}

				container {
					image = "oliver006/redis_exporter:v1.52.0"
					name = "redis-exporter"

					# TODO: How to make this cleaner? Ternary operator solely for last element in list
					args = each.key == "redis-chirp" ? [ 
						"--redis.addr=${each.value.endpoint}",
						"--redis.user=${module.redis_secrets.values["redis/${each.key}/username"]}",
						"--redis.password=${module.redis_secrets.values["redis/${each.key}/password"]}",
						"--check-streams=chirp:topic:*",
					] : [ 
						"--redis.addr=${each.value.endpoint}",
						"--redis.user=${module.redis_secrets.values["redis/${each.key}/username"]}",
						"--redis.password=${module.redis_secrets.values["redis/${each.key}/password"]}",
					]

					resources {
						limits = {
							memory = "${local.service_redis_exporter.resources.memory}Mi"
							cpu = local.service_redis_exporter.resources.cpu_cores > 0 ? "${local.service_redis_exporter.resources.cpu_cores * 1000}m" : "${local.service_redis_exporter.resources.cpu}m"
						}
					}

					port {
						name = "http"
						container_port = 9121
					}
				}
			}
		}
	}
}

resource "kubernetes_service" "redis_exporter" {
	for_each = var.redis_svcs

	metadata {
		name = "redis-exporter-${each.key}"
	}
	spec {
		selector = {
			app = kubernetes_deployment.redis_exporter[each.key].metadata.0.name
		}
		type = "ClusterIP"

		port {
			protocol = "TCP"
			port = 9121
			target_port = "http"
		}
	}
}
