resource "kubernetes_namespace" "vector" {
	metadata {
		name = "vector"
	}
}
