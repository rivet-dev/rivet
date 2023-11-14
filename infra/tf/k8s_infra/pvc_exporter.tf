resource "kubernetes_namespace" "pvc_exporter" {
	metadata {
		name = "pvc-exporter"
	}
}

# https://github.com/kais271/pvc-exporter
resource "helm_release" "pvc_exporter" {
	depends_on = [helm_release.prometheus]

	name = "pvc-exporter"
	namespace = kubernetes_namespace.pvc_exporter.metadata.0.name
	repository = "https://kais271.github.io/pvc-exporter/helm3/charts/"
	chart = "pvc-exporter"
	version = "0.1.4-beta"
	values = [yamlencode({
		PvcExporter = {
			resources = {
				requests = {
					cpu = "100m"
					memory = "100Mi"
				}
				limits = {
					cpu = "200m"
					memory = "200Mi"
				}
			}
		}
	})]
}
