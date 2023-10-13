# Useful: https://github.com/kubernetes/kube-state-metrics/blob/main/docs/pod-metrics.md
resource "kubectl_manifest" "pod_rules" {
	depends_on = [helm_release.prometheus]

	yaml_body = yamlencode({
		apiVersion = "monitoring.coreos.com/v1"
		kind = "PrometheusRule"
		metadata = {
			name = "pod-rules"
			namespace = kubernetes_namespace.prometheus.metadata.0.name
		}
		spec = {
			groups = [
				{
					name = "pod-health"
					interval = "15s"
					rules = [
						{
							alert = "PodHighMemoryUsage"
							annotations = {
								summary = "Pod High Memory usage ({{ $labels.namespace }}/{{ $labels.pod }})"
								description = "Pod Memory usage has been above 80% for over 2 minutes\n  VALUE = {{ printf \"%.2f%%\" $value }}\n  LABELS = {{ $labels }}"
							}
							# TODO: Maybe use kube_pod_container_resource_limits{resource="memory"} instead?
							expr = "(sum(container_memory_working_set_bytes{name!=\"\"}) BY (namespace, pod) / sum(container_spec_memory_limit_bytes > 0) BY (namespace, pod) * 100) > 80"
							"for" = "2m"
							labels = {
								severity = "warning"
							}
						},
						{
							alert = "PodHighThrottleRate"
							annotations = {
								summary = "Pod high throttle rate ({{ $labels.namespace }}/{{ $labels.pod }})"
								description = "Pod is being throttled\n  VALUE = {{ $value }}\n  LABELS = {{ $labels }}"
							}
							expr = "rate(container_cpu_cfs_throttled_seconds_total{job!=\"\"}[3m]) > 1"
							"for" = "2m"
							labels = {
								severity = "warning"
							}
						},
						{
							alert = "PodLowCpuUtilization"
							annotations = {
								summary = "Pod Low CPU utilization ({{ $labels.namespace }}/{{ $labels.pod }})"
								description = "Pod CPU utilization is under 20% for 1 week. Consider reducing the allocated CPU.\n  VALUE = {{ printf \"%.2f%%\" $value }}\n  LABELS = {{ $labels }}"
							}
							expr = "(sum(rate(container_cpu_usage_seconds_total{name!=\"\"}[3m])) BY (namespace, pod) * 100) < 20"
							"for" = "7d"
							labels = {
								severity = "info"
							}
						},
						{
							alert = "PodLowMemoryUsage"
							annotations = {
								summary = "Pod Low Memory usage ({{ $labels.namespace }}/{{ $labels.pod }})"
								description = "Pod Memory usage is under 20% for 1 week. Consider reducing the allocated memory.\n  VALUE = {{ printf \"%.2f%%\" $value }}\n  LABELS = {{ $labels }}"
							}
							expr = "(sum(container_memory_working_set_bytes{name!=\"\"}) BY (namespace, pod) / sum(container_spec_memory_limit_bytes > 0) BY (namespace, pod) * 100) < 20"
							"for" = "7d"
							labels = {
								severity = "info"
							}
						}
					]
				}
			]
		}
	})
}

resource "kubectl_manifest" "host_rules" {
	depends_on = [helm_release.prometheus]

	yaml_body = yamlencode({
		apiVersion = "monitoring.coreos.com/v1"
		kind = "PrometheusRule"
		metadata = {
			name = "host-rules"
			namespace = kubernetes_namespace.prometheus.metadata.0.name
		}
		spec = {
			groups = [
				{
					name = "host-health"
					interval = "30m"
					rules = [
						{
							alert = "HostOutOfDiskSpace"
							expr = "((node_filesystem_avail_bytes * 100) / node_filesystem_size_bytes < 10 and ON (instance, device, mountpoint) node_filesystem_readonly == 0) * on(instance) group_left (nodename) node_uname_info{nodename=~\".+\"}"
							"for" = "2m"
							labels = {
								severity = "warning"
							}
							annotations = {
								summary = "Host out of disk space (instance {{ $labels.instance }})"
								description = "Disk is almost full (< 10% left)\n  VALUE = {{ $value }}\n  LABELS = {{ $labels }}"
							}
						}
					]
				}
			]
		}
	})
}

resource "kubectl_manifest" "chirp_rules" {
	depends_on = [helm_release.prometheus]

	yaml_body = yamlencode({
		apiVersion = "monitoring.coreos.com/v1"
		kind = "PrometheusRule"
		metadata = {
			name = "chirp-rules"
			namespace = kubernetes_namespace.prometheus.metadata.0.name
		}
		spec = {
			groups = [
				{
					name = "chirp-health"
					interval = "1s"
					rules = [
						{
							alert = "ChirpHighErrorRate"
							annotations = {
								summary = "{{ $labels.service }} high error rate ({{ $value }})"
								description = "{{ $labels.service }} high error rate ({{ $value }})"
							}
							expr = <<-EOF
								(sum by (service)
								(increase(rivet_chirp_request_duration_count{error_code!="",error_code!~"(1002|VALIDATION_ERROR)"}
								[2m]))

								/

								sum by (service)
								(clamp_min(increase(rivet_chirp_request_duration_count{error_code!~"(1002|VALIDATION_ERROR)"}
								[2m]), 1))) > 0.05
								EOF
							labels = {
								severity = "warning"
							}
						}
					]
				}
			]
		}
	})
}


resource "kubectl_manifest" "api_rules" {
	depends_on = [helm_release.prometheus]

	yaml_body = yamlencode({
		apiVersion = "monitoring.coreos.com/v1"
		kind = "PrometheusRule"
		metadata = {
			name = "api-rules"
			namespace = kubernetes_namespace.prometheus.metadata.0.name
		}
		spec = {
			groups = [
				{
					name = "api-health"
					interval = "1s"
					rules = [
						{
							alert = "APIHighErrorRate"
							annotations = {
								summary = "{{ $labels.service }} high API error rate ({{ $value }})"
								description = "{{ $labels.service }} high API error rate ({{ $value }})"
							}
							expr = <<-EOF
								(sum by (service)
								(increase(rivet_api_request_duration_count{error_code!="",error_code!~"(API_CANCELLED|CAPTCHA_CAPTCHA_REQUIRED)"}
								[2m]))

								/

								sum by (service)
								(clamp_min(increase(rivet_api_request_duration_count{error_code!~"(API_CANCELLED|CAPTCHA_CAPTCHA_REQUIRED)"}
								[2m]), 1))) > 0.05
								EOF
							labels = {
								severity = "warning"
							}
						}
					]
				}
			]
		}
	})
}
