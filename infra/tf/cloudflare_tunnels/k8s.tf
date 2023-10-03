locals {
	tunnel = module.cloudflare_tunnels["local"]
}

resource "kubernetes_namespace" "cloudflared" {
	metadata {
		name = "cloudflared"
	}
}

resource "kubernetes_deployment" "cloudflared" {
	metadata {
		name = "cloudflared"
		namespace = kubernetes_namespace.cloudflared.metadata.0.name
	}

	spec {
		replicas = 1

		selector {
			match_labels = {
				"app.kubernetes.io/name" = "cloudflared"
			}
		}

		template {
			metadata {
				labels = {
					"app.kubernetes.io/name" = "cloudflared"
				}
			}

			spec {
				container {
					image = "cloudflare/cloudflared:2023.8.2"
					name = "cloudflared"

					args = [ "tunnel", "--config", "/etc/cloudflared/config/config.yaml", "run" ]
					
					liveness_probe {
						http_get {
							path = "/ready"
							port = 2000
						}
						
						failure_threshold = 1
						initial_delay_seconds = 10
						period_seconds = 10
					}

					volume_mount {
						name = "config"
						mount_path = "/etc/cloudflared/config"
						read_only = true
					}

					volume_mount {
						name = "creds"
						mount_path = "/etc/cloudflared/creds"
						read_only = true
					}
				}

				volume {
					name = "creds"

					secret {
						secret_name = "tunnel-credentials"
					}
				}

				volume {
					name = "config"

					config_map {
						name = "cloudflared"

						items {
							key = "config.yaml"
							path = "config.yaml"
						}
					}
				}
			}
		}
	}
}

resource "kubernetes_secret" "tunnel_credentials" {
	metadata {
		name = "tunnel-credentials"
		namespace = kubernetes_namespace.cloudflared.metadata.0.name
	}

	data = {
		"credentials.json" = jsonencode(local.tunnel.cert)
	}
}

resource "kubernetes_config_map" "cloudflared" {
	metadata {
		name = "cloudflared"
		namespace = kubernetes_namespace.cloudflared.metadata.0.name
	}

	data = {
		"config.yaml" = yamlencode({
			tunnel = local.tunnel.tunnel_name
			credentials-file = "/etc/cloudflared/creds/credentials.json"
			metrics = "0.0.0.0:2000"
			"no-autoupdate" = true
			ingress = local.tunnel.ingress
		})
	}
}
