resource "kubernetes_namespace_v1" "k8s_dashboard" {
	metadata {
		name = "kubernetes-dashboard"
	}
}

resource "helm_release" "k8s_dashboard" {
	depends_on = [kubernetes_namespace_v1.k8s_dashboard]

	name = "kubernetes-dashboard"
	namespace = "kubernetes-dashboard"
	repository = "https://kubernetes.github.io/dashboard/"
	chart = "kubernetes-dashboard"
	# Version 7 doesn't seem to work
	version = "6.0.8"
	values = [yamlencode({
		# Installed by default on k3s
		metrics-server = {
			enabled = false
		}
	})]
}

resource "kubernetes_service_account_v1" "example" {
	depends_on = [helm_release.k8s_dashboard]

	metadata {
		name = "admin-user"
		namespace = "kubernetes-dashboard"
	}
}

resource "kubernetes_cluster_role_binding_v1" "example" {
	depends_on = [helm_release.k8s_dashboard]

	metadata {
		name = "admin-user"
	}

	role_ref {
		api_group = "rbac.authorization.k8s.io"
		kind = "ClusterRole"
		name = "cluster-admin"
	}

	subject {
		kind = "ServiceAccount"
		name = "admin-user"
		namespace = "kubernetes-dashboard"
	}
}

