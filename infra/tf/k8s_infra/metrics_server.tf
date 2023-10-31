resource "kubernetes_namespace" "loki" {
	metadata {
		name = "metrics-server"
	}
}

# Deploy metrics server for distributed clusters.
#
# K3d comes with metrics-server preinstalled.
# 
# We don't run this in the kube-system namespace because EKS runs
# everything in the kube-system namespace on Fargate by default,
# which doesn't allow us to access the node API.
resource "helm_release" "metrics_server" {
	count = var.deploy_method_cluster ? 1 : 0

	name = "metrics-server"
	namespace = kubernetes_namespace.loki.metadata.0.name
	repository = "https://kubernetes-sigs.github.io/metrics-server/"
	chart = "metrics-server"
	version = "3.11.0"
	values = [yamlencode({
		resources = {
			limits = {
				cpu = "100m"
				memory = "200Mi"
			}
		}
	})]
}
