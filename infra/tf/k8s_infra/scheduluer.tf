resource "kubectl_manifest" "scheduler_config" {
	count = var.deploy_method_cluster ? 1 : 0

	yaml_body = yamlencode({
		apiVersion = "kubescheduler.config.k8s.io/v1"
		kind = "KubeSchedulerConfiguration"

		profiles = [{
			schedulerName = "default-scheduler"
			pluginConfig = [{
				name = "PodTopologySpread"
				args = {
					defaultConstraints = [
						{
							maxSkew = 1
							topologyKey = "kubernetes.io/zone"
							whenUnsatisfiable = "ScheduleAnyway"
						},
						{
							maxSkew = 1
							topologyKey = "topology.kubernetes.io/zone"
							whenUnsatisfiable = "ScheduleAnyway"
						}
					]
					defaultingType = "List"
				}
			}]
		}]
	})
}
