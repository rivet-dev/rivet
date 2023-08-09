resource "kubernetes_namespace" "k8s_dashboard" {
	metadata {
		name = "kubernetes-dashboard"
	}
}

resource "helm_release" "k8s_dashboard" {
	depends_on = [kubernetes_namespace.k8s_dashboard]

	name = "k8s_dashboard"
	namespace = "k8s_dashboard"
	repository = "https://kubernetes.github.io/dashboard/"
	chart = "kubernetes-dashboard"
	version = "7.0.3"
}

