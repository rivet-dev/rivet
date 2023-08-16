resource "kubernetes_namespace" "clickhouse" {
	metadata {
		name = "clickhouse"
	}
}

resource "helm_release" "clickhouse" {
	name = "clickhouse"
	namespace = kubernetes_namespace.clickhouse.metadata.0.name
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

