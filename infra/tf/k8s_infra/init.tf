resource "kubernetes_namespace" "rivet_service" {
	metadata {
		name = "rivet-service"
	}
}

# Used by shells and migrations
resource "kubernetes_namespace" "bolt" {
	metadata {
		name = "bolt"
	}
}

module "docker_auth" {
	source = "../modules/k8s_auth"

	namespaces = [
		for x in [
			kubernetes_namespace.traffic_server,
			# kubernetes_namespace.redis_exporter,
			kubernetes_namespace.rivet_service,
			kubernetes_namespace.imagor,
			kubernetes_namespace.nsfw_api
		]:
		x.metadata.0.name
	]
	authenticate_all_docker_hub_pulls = var.authenticate_all_docker_hub_pulls
	deploy_method_cluster = var.deploy_method_cluster
}
