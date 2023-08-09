resource "kubernetes_namespace_v1" "clickhouse" {
	metadata {
		name = "clickhouse"
	}
}

resource "helm_release" "clickhouse" {
	depends_on = [kubernetes_namespace_v1.clickhouse]

	name = "clickhouse"
	namespace = "clickhouse"
	repository = "oci://registry-1.docker.io/bitnamicharts"
	chart = "clickhouse"
	version = "3.6.3"
	values = [yamlencode({
		replicaCount = 1
		shards = 1
		zookeeper = {
			replicaCount = 1
		}
	})]
}

