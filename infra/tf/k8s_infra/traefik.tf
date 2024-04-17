resource "kubernetes_namespace" "traefik" {
	metadata {
		name = "traefik"
	}
}

module "traefik_secrets" {
	source = "../modules/secrets"

	keys = [
		"rivet/api_traefik_provider/token",
	]
}

locals {
	service_traefik = lookup(var.services, "traefik", {
		count = var.deploy_method_cluster ? 2 : 1
		resources = {
			cpu = 1000
			memory = 2048
		}
	})
}

resource "kubernetes_priority_class" "traefik_priority" {
	metadata {
		name = "traefik-priority"
	}
	value = 40
}

resource "helm_release" "traefik" {
	depends_on = [null_resource.daemons]

	name = "traefik"
	namespace = kubernetes_namespace.traefik.metadata.0.name
	repository = "https://traefik.github.io/charts"
	chart = "traefik"
	version = "24.0.0"
	values = [yamlencode({
		# Upgrade image to fix CVEs
		image = {
			tag = "v2.10.7"
		}

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

		priorityClassName = kubernetes_priority_class.traefik_priority.metadata.0.name
		resources = var.limit_resources ? {
			limits = {
				memory = "${local.service_traefik.resources.memory}Mi"
				cpu = "${local.service_traefik.resources.cpu}m"
			}
		} : null

		additionalArguments = [
			"--providers.http.endpoint=http://rivet-api-internal-monolith.rivet-service.svc.cluster.local/traefik-provider/config/core?token=${module.traefik_secrets.values["rivet/api_traefik_provider/token"]}",
			"--providers.http.pollInterval=2.5s",
			# See docs/infrastructure/TIMEOUTS.md
			"--entryPoints.web.transport.lifeCycle.graceTimeOut=60s",
			"--entryPoints.websecure.transport.lifeCycle.graceTimeOut=60s",
			"--entryPoints.web.transport.respondingTimeouts.readTimeout=120s",
			"--entryPoints.websecure.transport.respondingTimeouts.readTimeout=120s",
			"--entryPoints.web.transport.respondingTimeouts.writeTimeout=120s",
			"--entryPoints.websecure.transport.respondingTimeouts.writeTimeout=120s",
		]

		logs = {
			general = {
				level = "DEBUG"
			}
			access = {
				enabled = true
			}
		}

		deployment = {
			replicas = local.service_traefik.count
		}

		service = {
			enabled = true
			annotations = var.deploy_method_cluster ? {
				# See: https://docs.aws.amazon.com/eks/latest/userguide/network-load-balancing.html
				"service.beta.kubernetes.io/aws-load-balancer-type" = "external"
				# Removes the need for an extra network hop: https://kubernetes-sigs.github.io/aws-load-balancer-controller/v2.2/guide/service/nlb/#ip-mode
				"service.beta.kubernetes.io/aws-load-balancer-nlb-target-type" = "ip"
				"service.beta.kubernetes.io/aws-load-balancer-scheme" = "internet-facing"
			} : {}
		}

		autoscaling = {
			enabled = var.deploy_method_cluster
			minReplicas = local.service_traefik.count
			maxReplicas = 10
			metrics = [
				{
					type = "Resource"
					resource = {
						name = "cpu"
						target = {
							type = "Utilization"
							averageUtilization = 60
						}
					}
				},
				{
					type = "Resource"
					resource = {
						name = "memory"
						target = {
							type = "Utilization"
							averageUtilization = 60
						}
					}
				},
			]
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
				addEntryPointsLabels = false
				addRoutersLabels = false  # There will be a lot of CDN routers
				addServicesLabels = true
				# See lib/chirp/metrics/src/buckets.rs
				buckets = "0.001,0.0025,0.005,0.01,0.025,0.05,0.1,0.25,0.5,1.0,2.5,5.0,10.0,25.0,50.0,100.0"
			}
		}
	})]
}

resource "kubernetes_service" "traefik_headless" {
	depends_on = [helm_release.traefik]

	metadata {
		name = "traefik-headless"
		namespace = kubernetes_namespace.traefik.metadata.0.name
		labels = {
			"app.kubernetes.io/name" = "traefik-headless"
		}
	}

	spec {
		selector = {
			"app.kubernetes.io/name" = "traefik"
		}

		cluster_ip = "None"

		port {
			name = "web"
			port = 80
			target_port = "web"
		}

		port {
			name = "websecure"
			port = 443
			target_port = "websecure"
		}

		port {
			name = "traefik"
			port = 9000
			target_port = "traefik"
		}

		port {
			name = "metrics"
			port = 9100
			target_port = "metrics"
		}
	}
}

resource "kubectl_manifest" "traefik_service_monitor" {
	depends_on = [helm_release.traefik]

	yaml_body = yamlencode({
		apiVersion = "monitoring.coreos.com/v1"
		kind = "ServiceMonitor"

		metadata = {
			name = "traefik-service-monitor"
			namespace = kubernetes_namespace.traefik.metadata.0.name
		}

		spec = {
			selector = {
				matchLabels = {
					"app.kubernetes.io/name": "traefik-headless"
				}
			}
			endpoints = [
				{
					port = "metrics"
					path = "/metrics"
				}
			]
		}
	})
}

data "kubernetes_service" "traefik" {
	depends_on = [helm_release.traefik]

	metadata {
		name = "traefik"
		namespace = kubernetes_namespace.traefik.metadata.0.name
	}
}

