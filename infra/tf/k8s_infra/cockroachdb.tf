resource "kubernetes_namespace" "cockroachdb" {
	metadata {
		name = "clickhouse"
	}
}

resource "helm_release" "cockroachdb" {
	depends_on = [kubernetes_namespace.cockroachdb]

	name = "cockroachdb"
	namespace = "cockroachdb"
	repository = "https://charts.cockroachdb.com/"
	chart = "cockroachdb"
	version = "11.1.5"
}

