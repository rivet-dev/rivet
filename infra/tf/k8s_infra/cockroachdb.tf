resource "kubernetes_namespace_v1" "cockroachdb" {
	metadata {
		name = "cockroachdb"
	}
}

resource "helm_release" "cockroachdb" {
	depends_on = [kubernetes_namespace_v1.cockroachdb]

	name = "cockroachdb"
	namespace = "cockroachdb"
	repository = "https://charts.cockroachdb.com/"
	chart = "cockroachdb"
	version = "11.1.5"
	values = [yamlencode({
		conf = {
			single-node = true
			statefulset = {
				replicas = 3
			}
		}
	})]
}

