resource "kubernetes_namespace" "vpa" {
	count = var.limit_resources ? 1 : 0

	metadata {
		name = "vpa"
	}
}

resource "helm_release" "vpa" {
	count = var.limit_resources ? 1 : 0

	name = "vpa"
	repository = "https://charts.fairwinds.com/stable"
	namespace = kubernetes_namespace.vpa[0].metadata.0.name
	chart = "vpa"
	version = "3.0.2"
}
