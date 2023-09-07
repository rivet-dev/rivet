resource "kubernetes_namespace" "cloudflare" {
	metadata {
		name = "cloudflare"
	}
}

resource "kubernetes_deployment" "cloudflare" {
	depends_on = [kubernetes_namespace.cloudflare]
	
	metadata {
		name = "cloudflared"
		namespace = "cloudflare"
	}

	spec {
		replicas = 1

		selector {
			match_labels = {
				"app.kubernetes.io/name" = "cloudflare"
			}
		}

		template {
			metadata {
				labels = {
					"app.kubernetes.io/name" = "cloudflare"
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
	depends_on = [kubernetes_namespace.cloudflare]

	metadata {
		name = "tunnel-credentials"
		namespace = "cloudflare"
	}

	data = {
		"credentials.json" = data.terraform_remote_state.tls.outputs.tls_cert_cloudflare_ca
	}
}

resource "kubernetes_config_map" "cloudflared" {
	depends_on = [kubernetes_namespace.cloudflare]

	metadata {
		name = "cloudflared"
		namespace = "cloudflare"
	}

	data = {
		"config.yaml" = yamlencode({
			tunnel = "example-tunnel"
			credentials-file = "/etc/cloudflared/creds/credentials.json"
			metrics = "0.0.0.0:2000"
			"no-autoupdate" = true
			ingress = [{
				hostname = ""
				service = ""
			}]
		})
	}
}