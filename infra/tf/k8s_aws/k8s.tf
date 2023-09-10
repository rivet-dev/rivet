provider "kubernetes" {
	host = module.eks.cluster_endpoint
	cluster_ca_certificate = base64decode(module.eks.cluster_certificate_authority_data)

	# Dynamically access token
	exec {
		api_version = "client.authentication.k8s.io/v1beta1"
		command = "aws"
		args = ["eks", "get-token", "--cluster-name", module.eks.cluster_name]
	}
}

resource "kubernetes_namespace" "app_test" {
	metadata {
		name = "app-test"
	}
}

resource "kubernetes_namespace" "backend" {
	metadata {
		name = "backend"
	}
}

resource "kubernetes_namespace" "test_ns" {
	metadata {
		name = "test-ns"
	}
}

# resource "kubernetes_pod" "hello_world_pod" {
# 	metadata {
# 		name = "hello-world-pod"
# 		namespace = kubernetes_namespace.backend.metadata.0.name

# 		annotations = {
# 			# CapacityProvisioned = "0.25vCPU 0.5GB"
# 			# "eks.amazonaws.com/compute-type" = "Fargate"
# 		}
# 	}

# 	spec {
# 		container {
# 			image = "busybox"
# 			name  = "hello-world-container"

# 			command = ["sh", "-c", "while true; do echo 'Hello, World!'; sleep 1; done"]
# 		}
# 	}
# }

resource "kubernetes_deployment" "example" {
	metadata {
		name = "nginx-deployment"
		namespace = kubernetes_namespace.test_ns.metadata.0.name
		labels = {
			app = "nginx"
		}
	}

	spec {
		replicas = 2

		selector {
			match_labels = {
				app = "nginx"
			}
		}

		template {
			metadata {
				labels = {
					app = "nginx"
				}
			}

			spec {
				container {
					name  = "nginx"
					image = "nginx:1.19.3"

					port {
						container_port = 80
					}

					resources {}
				}
			}
		}
	}
}
