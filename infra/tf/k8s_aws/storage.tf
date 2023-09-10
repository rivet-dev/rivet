resource "kubernetes_storage_class" "efs_sc" {
	metadata {
		name = "efs-sc"
	}
	storage_provisioner = "efs.csi.aws.com"
	allow_volume_expansion = true
	reclaim_policy = "Retain"
}

