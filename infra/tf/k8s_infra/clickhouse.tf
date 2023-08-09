resource "kubernetes_namespace" "clickhouse" {
	metadata {
		name = "clickhouse"
	}
}

resource "helm_release" "clickhouse" {
	depends_on = [kubernetes_namespace.clickhouse]

	name = "clickhouse"
	namespace = "clickhouse"
	repository = "oci://registry-1.docker.io/bitnamicharts"
	chart = "clickhouse"
	version = "3.6.3"
}

