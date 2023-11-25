# NOTE: If adding a new rule results in an `Rule invalid" error from terraform apply, check the logs of
# prometheus-operator for the actual cause.

locals {
	pod_rules = [
		{
			alert = "PodHighMemoryUsage"
			annotations = {
				summary = "Pod High Memory usage ({{ $labels.namespace }}/{{ $labels.pod }})"
				description = "Pod Memory usage has been above 80% for over 2 minutes\n  VALUE = {{ printf \"%.2f%%\" $value }}\n  LABELS = {{ $labels }}"
			}
			# Exclude Prometheus because it intentionally maintains high memory usage as a cache
			expr = <<-EOF
				(
					sum(container_memory_working_set_bytes{name!="", container!="prometheus"})
					BY (namespace, pod)
					/
					sum(kube_pod_container_resource_limits{resource="memory", container!="prometheus"} > 0)
					BY (namespace, pod)
				) * 100
				> 80
				EOF
			"for" = "2m"
			labels = {
				severity = "warning"
			}
		},
		{
			alert = "PodHighCpuUtilization"
			annotations = {
				summary = "Pod High CPU utilization ({{ $labels.namespace }}/{{ $labels.pod }})"
			}
			expr = "(sum(rate(container_cpu_usage_seconds_total{name!=\"\"}[3m])) BY (namespace, pod) * 100) > 90"
			"for" = "5m"
			labels = {
				severity = "info"
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
			alert = "PodLowMemoryUsage"
			annotations = {
				summary = "Pod Low Memory usage ({{ $labels.namespace }}/{{ $labels.pod }})"
				description = "Pod Memory usage is under 20% for 1 week. Consider reducing the allocated memory.\n  VALUE = {{ printf \"%.2f%%\" $value }}\n  LABELS = {{ $labels }}"
			}
			expr = <<-EOF
				(
					sum(container_memory_working_set_bytes{name!=""})
					BY (namespace, pod)
					/
					sum(kube_pod_container_resource_limits{resource="memory"} > 0)
					BY (namespace, pod)
				) * 100
				< 20
				EOF
			"for" = "7d"
			labels = {
				severity = "info"
			}
		}
	]
}

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
					rules = (var.deploy_method_cluster ?
						concat(local.pod_rules, [{
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
						}]) :
						local.pod_rules
					)
				}
			]
		}
	})
}

resource "kubectl_manifest" "pvc_rules" {
	depends_on = [helm_release.prometheus]

	yaml_body = yamlencode({
		apiVersion = "monitoring.coreos.com/v1"
		kind = "PrometheusRule"
		metadata = {
			name = "persistent-volume-claim-rules"
			namespace = kubernetes_namespace.prometheus.metadata.0.name
		}
		spec = {
			groups = [
				{
					name = "pvc-health"
					interval = "1m"
					rules = [
						{
							alert = "PVCHighDiskUsage"
							annotations = {
								summary = "Persistent volume claim almost full ({{ $labels.namespace }}/{{ $labels.persistentvolumeclaim }})"
								description = "Persistent volume claim almost full ({{ printf \"%.2f%%\" $value }}) ({{ $labels.namespace }}/{{ $labels.persistentvolumeclaim }})"
							}
							expr = "(kubelet_volume_stats_used_bytes / kubelet_volume_stats_capacity_bytes) * 100 > 75"
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
							alert = "HostHighDiskUsage"
							annotations = {
								summary = "Host disk almost full (instance {{ $labels.instance }})"
								description = "Disk is almost full (< 10% left)\n  VALUE = {{ $value }}\n  LABELS = {{ $labels }}"
							}
							expr = <<-EOF
								(
									(node_filesystem_avail_bytes * 100) / node_filesystem_size_bytes < 10
									AND
									ON (instance, device, mountpoint) node_filesystem_readonly == 0
								)
								*
								ON(instance)
								group_left (nodename) node_uname_info{nodename=~".+"}
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
						},
						{
							alert = "ChirpHighPendingRequests"
							annotations = {
								summary = "High pending requests ({{ $labels.group }} $value)"
								description = "High pending requests ({{ $labels.group }} $value)"
							}
							expr = "max(redis_stream_group_messages_pending) by (group) > 100"
							"for" = "15s"
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
						},
						{
							alert = "ApiHighRequestDuration"
							annotations = {
								summary = "High request duration ({{ $labels.service }})"
								description = "Request duration above 1s ({{ $labels.service }} {{ $labels.method }} {{ $labels.path }})"
							}
							expr = <<-EOF
								sum by (service, path, method) (
									increase(
										rivet_api_request_duration_bucket{
											watch="0",
											path!~"/find|/create",
											service!="rivet-api-route",
											le="+Inf"
										} [2m]
									)
									-
									on(service, path, method, status, kubernetes_pod_id)
									increase(
										rivet_api_request_duration_bucket{
											watch="0",
											path!~"/find|/create",
											service!="rivet-api-route",
											le="1"
										} [2m]
									)
								) > 0
								EOF
							labels = {
								severity = "warning"
							}
						},
						{
							alert = "ApiHighPendingRequests"
							expr = <<-EOF
								sum(
									max_over_time(
										rivet_api_request_pending{watch="0"} [2m]
									)
								) by (service, path)
								> 100
								EOF
							"for" = "10s"
							labels = {
								severity = "warning"
							}
							annotations = {
								summary = "High pending requests ({{ $labels.service }} $value)"
								description = "High pending requests ({{ $labels.service }} {{ $labels.path }} $value)"
							}
						}
					]
				}
			]
		}
	})
}

resource "kubectl_manifest" "crdb_rules" {
	depends_on = [helm_release.prometheus]

	yaml_body = yamlencode({
		apiVersion = "monitoring.coreos.com/v1"
		kind = "PrometheusRule"
		metadata = {
			name = "crdb-rules"
			namespace = kubernetes_namespace.prometheus.metadata.0.name
		}
		spec = {
			groups = [
				{
					name = "crdb-health"
					interval = "1m"
					rules = [
						{
							alert = "CrdbHighActivePools"
							expr = <<-EOF
								sum(
									max_over_time(
										rivet_crdb_pool_conn_size [2m]
									)
									- max_over_time(
										rivet_crdb_pool_num_idle [2m]
									)
								) by (service, db_name)
								> 20
								EOF
							labels = {
								severity = "warning"
							}
							annotations = {
								summary = "High active CRDB pools ({{ $labels.service }} $value)"
								description = "High active CRDB pools (service {{ $labels.service }}, db {{ $labels.db_name }}, value $value)"
							}
						}
					]
				}
			]
		}
	})
}

resource "kubectl_manifest" "nomad_rules" {
	depends_on = [helm_release.prometheus]

	yaml_body = yamlencode({
		apiVersion = "monitoring.coreos.com/v1"
		kind = "PrometheusRule"
		metadata = {
			name = "nomad-rules"
			namespace = kubernetes_namespace.prometheus.metadata.0.name
		}
		spec = {
			groups = [
				{
					name = "nomad-health"
					interval = "1m"
					rules = [
						{
							alert = "NomadHighMemoryAllocated"
							expr = <<-EOF
								sum by (datacenter) (nomad_client_allocated_memory{node_class=~"job"}) /
								sum by (datacenter) (nomad_client_allocated_memory{node_class=~"job"} + nomad_client_unallocated_memory{node_class=~"job"})
								> 0.8
								EOF
							"for" = "5m"
							labels = {
								severity = "warning"
							}
						},
						{
							alert = "NomadHighCpuAllocated"
							expr = <<-EOF
								sum by (datacenter) (nomad_client_allocated_cpu{node_class=~"job"}) /
								sum by (datacenter) (nomad_client_allocated_cpu{node_class=~"job"} + nomad_client_unallocated_cpu{node_class=~"job"})
								> 0.8
								EOF
							"for" = "5m"
							labels = {
								severity = "warning"
							}
						},
						{
							alert = "NomadBlockedEvalulation"
							expr = <<-EOF
								nomad_nomad_blocked_evals_total_blocked > 0
								EOF
							labels = {
								severity = "warning"
							}
							annotations = {
								summary = "Nomad blocked evaluation for job {{ $labels.exported_job }} (instance {{ $labels.instance }})"
							}
						},
						{
							alert = "NomadJobQueued"
							expr = <<-EOF
								nomad_nomad_job_summary_queued > 0
								EOF
							"for" = "5m"
							labels = {
								severity = "warning"
							}
							annotations = {
								summary = "Nomad job {{ $labels.exported_job }} queued (instance {{ $labels.instance }})"
							}
						},
						{
							alert = "NomadJobLost"
							expr = <<-EOF
								nomad_nomad_job_summary_lost > 0
								EOF
							"for" = "5m"
							labels = {
								severity = "warning"
							}
							annotations = {
								summary = "Nomad job {{ $labels.exported_job }} lost (instance {{ $labels.instance }})"
							}
						},
						{
							alert = "NomadJobFailed"
							expr = <<-EOF
								nomad_nomad_job_summary_failed{exported_job!~"job-.*",exported_job!~".*/periodic-.*"} > 0
								EOF
							"for" = "5m"
							labels = {
								severity = "warning"
							}
							annotations = {
								summary = "Nomad job {{ $labels.exported_job }} failed (instance {{ $labels.instance }})"
							}
						}
					]
				}
			]
		}
	})
}

resource "kubectl_manifest" "traefik_rules" {
	depends_on = [helm_release.prometheus]

	yaml_body = yamlencode({
		apiVersion = "monitoring.coreos.com/v1"
		kind = "PrometheusRule"
		metadata = {
			name = "traefik-rules"
			namespace = kubernetes_namespace.prometheus.metadata.0.name
		}
		spec = {
			groups = [
				{
					name = "traefik-health"
					interval = "1m"
					rules = [
						{
							alert = "TraefikHighHttp4xxErrorRateService"
							expr = <<-EOF
								sum(rate(traefik_service_requests_total{code=~"4\\d\\d", service!~"job-.*"}[2m])) by (service)
								/
								sum(clamp_min(rate(traefik_service_requests_total{service!~"job-.*"}[2m]), 1)) by (service)
								> 0.05
								EOF
							"for" = "5m"
							labels = {
								severity = "warning"
							}
							annotations = {
								summary = "Traefik high HTTP 4xx error rate service (instance {{ $labels.instance }})"
								description = "Traefik service 4xx error rate is above 5%"
							}
						},
						{
							alert = "TraefikHighHttp5xxErrorRateService"
							expr = <<-EOF
								sum(rate(traefik_service_requests_total{code=~"5\\d\\d", service!~"^job-.*"}[2m])) by (service)
								/
								sum(clamp_min(rate(traefik_service_requests_total{service!~"^job-.*"}[2m]), 1)) by (service)
								> 0.05
								EOF
							"for" = "5m"
							labels = {
								severity = "warning"
							}
							annotations = {
								summary = "Traefik high HTTP 5xx error rate service (instance {{ $labels.instance }})"
								description = "Traefik service 5xx error rate is above 5%"
							}
						},
					]
				}
			]
		}
	})
}
