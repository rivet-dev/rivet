
resource "kubernetes_namespace" "app_test" {
	metadata {
		name = "app-test"
	}
}

resource "kubernetes_namespace" "backend" {
	metadata {
		name = "backend"
	}
}

resource "kubernetes_namespace" "test_ns" {
	metadata {
		name = "test-ns"
	}
}

# TODO: Storage class

resource "kubernetes_persistent_volume" "example_pv" {
	metadata {
		name = "my-pv"
	}

	spec {
		capacity = {
			storage = "10Gi"
		}
		volume_mode = "Filesystem"
		access_modes = ["ReadWriteMany"]
		persistent_volume_reclaim_policy = "Retain"
		storage_class_name = kubernetes_storage_class.efs_sc.metadata.0.name
		persistent_volume_source {
			csi {
				driver = "efs.csi.aws.com"
				volume_handle = "fs-01e83fe2a11ae8419"
			}
		}
	}
}

resource "kubernetes_persistent_volume_claim" "example_pvc" {
	metadata {
		name      = "my-pvc"
		# namespace = kubernetes_namespace.test_ns.metadata[0].name
	}
	spec {
		access_modes = ["ReadWriteMany"]
		storage_class_name = kubernetes_storage_class.efs_sc.metadata.0.name
		volume_name = kubernetes_persistent_volume.example_pv.metadata.0.name
		resources {
			requests = {
				storage = "10Gi"
			}
		}
	}
}

# resource "kubernetes_pod" "example_pod" {
# 	metadata {
# 		name      = "hello-world-pod"
# 		namespace = kubernetes_namespace.test_ns.metadata[0].name
# 	}
# 	spec {
# 		container {
# 			image = "busybox"
# 			name  = "hello-container"
# 			command = ["/bin/sh", "-c", "echo Hello, World! > /mnt/hello_world.txt; sleep 999999"]

# 			volume_mount {
# 				name       = "hello-world-storage"
# 				mount_path = "/mnt"
# 			}
# 		}

# 		volume {
# 			name = "hello-world-storage"
# 			persistent_volume_claim {
# 				claim_name = kubernetes_persistent_volume_claim.example_pvc.metadata[0].name
# 			}
# 		}
# 	}
# }

