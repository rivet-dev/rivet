resource "kubernetes_namespace" "k8s_dashboard" {
	count = var.k8s_dashboard_enabled ? 1 : 0

	metadata {
		name = "kubernetes-dashboard"
	}
}

resource "helm_release" "k8s_dashboard" {
	count = var.k8s_dashboard_enabled ? 1 : 0
	depends_on = [null_resource.daemons]

	name = "kubernetes-dashboard"
	namespace = kubernetes_namespace.k8s_dashboard.0.metadata.0.name
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

resource "kubernetes_service_account" "admin_user" {
	count = var.k8s_dashboard_enabled ? 1 : 0

	metadata {
		namespace = kubernetes_namespace.k8s_dashboard.0.metadata.0.name
		name = "admin-user"
	}
}

resource "kubernetes_cluster_role_binding" "admin_user" {
	count = var.k8s_dashboard_enabled ? 1 : 0

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
		namespace = kubernetes_namespace.k8s_dashboard.0.metadata.0.name
		name = "admin-user"
	}
}

