# Define low-priority deployments to maintain unallocated capacity on Karpenter
# cluster to prevent premption.
#
# Higher replicas + lower resoruces = more smaller pods can preempt
# Lower replicas + higher resources = larger pod can preempt, but will require bigger preempted "wedges"
#
# https://aws.amazon.com/blogs/containers/eliminate-kubernetes-node-scaling-lag-with-pod-priority-and-over-provisioning/

resource "kubernetes_namespace" "karpenter_overprovision" {
	metadata {
		name = "karpenter-overprovision"
	}
}

resource "kubernetes_priority_class" "overprovision_priority" {
	metadata {
		name = "overprovision-priority"
	}
	value = -1
}

resource "kubernetes_deployment" "overprovision" {
	metadata {
		name = "overprovision"
		namespace = kubernetes_namespace.karpenter_overprovision.metadata.0.name
		labels = {
			app = "overprovisioning"
		}
	}

	spec {
		replicas = 2

		selector {
			match_labels = {
				app = "overprovisioning"
			}
		}

		template {
			metadata {
				labels = {
					app = "overprovisioning"
				}
			}

			spec {
				container {
					name = "pause"
					image = "registry.k8s.io/pause"

					resources {
						requests = {
							cpu	= "1"
							memory = "500Mi"
						}

						limits = {
							cpu = "1"
							memory = "500Mi"
						}
					}
				}

				priority_class_name = kubernetes_priority_class.overprovision_priority.metadata.0.name
			}
		}
	}
}

