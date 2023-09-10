resource "kubernetes_csi_driver_v1" "efs" {
	metadata {
		name = "efs.csi.aws.com"
	}

	spec {
		attach_required = false
		volume_lifecycle_modes = ["Persistent"]
	}
}

resource "kubernetes_storage_class" "efs_sc" {
	metadata {
		name = "efs-sc"
	}
	storage_provisioner = kubernetes_csi_driver_v1.efs.metadata.0.name
	allow_volume_expansion = true
	reclaim_policy = "Retain"
}

