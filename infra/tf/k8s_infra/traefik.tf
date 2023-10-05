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
	namespace = kubernetes_namespace.traefik.metadata.0.name

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
			"--providers.http.endpoint=http://rivet-api-route.rivet-service.svc.cluster.local/traefik/config/core?token=${module.traefik_secrets.values["rivet/api_route/token"]}",
			"--providers.http.pollInterval=2.5s",
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

resource "kubernetes_namespace" "traefik_tunnel" {
	metadata {
		name = "traefik-tunnel"
	}
}


data "kubernetes_service" "traefik_tunnel" {
	depends_on = [helm_release.traefik_tunnel]

	metadata {
		name = "traefik-tunnel"
		namespace = kubernetes_namespace.traefik_tunnel.metadata.0.name
	}
}

# reuse secrets from existing Traefik instance - no need to re-define

# need to adjust config for second instance of traefik? 
resource "helm_release" "traefik_tunnel" {
	name = "traefik-tunnel"
	namespace = kubernetes_namespace.traefik_tunnel.metadata.0.name

	repository = "https://traefik.github.io/charts"
	chart = "traefik"
	values = [yamlencode({
		ports = {
			web = {
				expose = false
			},
			websecure = {
				expose = false
			},
			nomad = {
				port = 5000
				expose = true
			},
			api-route = {
				port = 5001
				expose = true
			}
		}

		# Allows referencing services outside of the traefik namespace
		# TODO eventually just specify the namespace(s) that are relevant so that not pulling in configs unncessarily
		providers = {
			kubernetesCRD = {
				allowCrossNamespace = true
			}
		}

		# TODO: specify static config here? or specify it using yaml? 
		additionalArguments = [
			"--entryPoints.nomad.address=:5000",
			"--entryPoints.api-route.address=:5001",
			"--providers.http.pollInterval=2.5s",
		]

# 		logs = {
# 			# general = {
# 			# 	level = "DEBUG"
# 			# }
# 			# NOTE: Do not enable on prod
# 			# access = {
# 			# 	enabled = true
# 			# }
# 		}

# 		metrics = {
# 			prometheus = {
# 				addEntryPointsLabels = true
# 				addRoutersLabels = true
# 				addServicesLabels = true
# 				# See lib/chirp/metrics/src/buckets.rs
# 				buckets = "0.001,0.0025,0.005,0.01,0.025,0.05,0.1,0.25,0.5,1.0,2.5,5.0,10.0,25.0,50.0,100.0"
# 			}
# 		}
	})]
}

# Q: why define it this way as opposed to defining it using yaml? 
# no need for dynamic custom config
# TODO: need to override the default port numbers which traefik exposes 
resource "kubectl_manifest" "traefik_tunnel" { # changed from data to resource - is this correct? 
	depends_on = [helm_release.traefik_tunnel]

	yaml_body = yamlencode({
		apiVersion = "traefik.containo.us/v1alpha1"
		kind = "TraefikService"

		metadata = {
			name = "traefik-tunnel"
			namespace = kubernetes_namespace.traefik_tunnel.metadata[0].name
		}

		spec = { # do we need to do traffic mirroring? if so, what port to use? 
			mirroring = {
				name = "traefik-tunnel"
				namespace = kubernetes_namespace.traefik_tunnel.metadata[0].name
				port = 8000
			}
		}
	})
}

# TODO: Create 2 instances of this for each service
resource "kubectl_manifest" "traefik_nomad_router" {
	depends_on = [helm_release.traefik_tunnel]

	yaml_body = yamlencode({
		apiVersion = "traefik.containo.us/v1alpha1"
		kind = "IngressRouteTCP"

		metadata = {
			name = "traefik-nomad-router"
			namespace = kubernetes_namespace.traefik_tunnel.metadata[0].name
		}


		spec = {
			entryPoints = [ "nomad" ]

			# TODO: how to port the dynamic config for api-route to static config? 

			# for nomad, what sorts of routes do we need to define? 

			routes = [
				{
					kind = "Rule"
					match = "HostSNI(`*`)" # TODO change to port mapping so that we don't need to configure dns stuff
					services = [
						{
							name = "nomad-server"
							port = 4646
						}
					]
				}
			]

			# do we need certResolver, domains, and passthrough?
			# is this the config for mtls? 
			tls = {
				secretName = "ingress-tls-cert-tunnel-server"
				options = {
					name = "ingress-tls-cert-tunnel-server"
					namespace = kubernetes_namespace.traefik_tunnel.metadata[0].name # TODO rename the secret 
				}
			}
		}
	})
}


# TODO add middleware? and configuration for api-route. currently, only stuff for nomad is configured
# MARK: Middleware


resource "kubectl_manifest" "traefik_api_route_router" {
	depends_on = [helm_release.traefik_tunnel]

	yaml_body = yamlencode({
		apiVersion = "traefik.containo.us/v1alpha1"
		kind = "IngressRouteTCP" # q: what other diff parameters do we need to configure for tcp (vs http)? 

		metadata = {
			name = "traefik-api-route-router"
			namespace = kubernetes_namespace.traefik_tunnel.metadata[0].name
		}


		spec = {
			entryPoints = [ "api-route" ]

			# TODO: how to port the dynamic config for api-route to static config? 

			# for nomad, what sorts of routes do we need to define? 

			routes = [
				{
					kind = "Rule"
					match = "HostSNI(`*`)" # TODO change to port mapping so that we don't need to configure dns stuff
					services = [
						{
							name = "nomad-server"
							port = 4646
						}
					]
				}
			]

			# do we need certResolver, domains, and passthrough?
			tls = {
				secretName = "ingress_tls_cert_tunnel_server"
				options = {
					name = "ingress-tls-cert-tunnel-server"
					namespace = kubernetes_namespace.traefik_tunnel.metadata[0].name
				}
			}
		}
	})
}

# NOTE: Must use kubectl_manifest because kubernetes_manifest doesn't work with CRDs. If this stops working
# correctly replace with a raw helm chart: https://artifacthub.io/packages/helm/wikimedia/raw
# https://github.com/hashicorp/terraform-provider-kubernetes/issues/1367#
resource "kubectl_manifest" "ingress_tls_tunnel" {
	depends_on = [helm_release.traefik_tunnel, kubernetes_namespace.traefik_tunnel]

	yaml_body = yamlencode({
		apiVersion = "traefik.containo.us/v1alpha1"
		kind = "TLSOption"

		metadata = {
			name = "ingress-tls-tunnel-server"
			namespace = kubernetes_namespace.traefik_tunnel.metadata.0.name
		}

		spec = {
			curvePreferences = [ "CurveP384" ]

			clientAuth = {
				secretNames = [ "ingress_tls_cert_tunnel_server" ]
				clientAuthType = "RequireAndVerifyClientCert"
			}
		}
	})
}