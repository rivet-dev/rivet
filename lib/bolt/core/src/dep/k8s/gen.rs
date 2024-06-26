use std::{
	collections::HashMap,
	path::{Path, PathBuf},
};

use anyhow::Result;
use duct::cmd;
use indexmap::IndexSet;
use serde_json::json;
use sha2::{Digest, Sha256};
use tokio::{fs, task::block_in_place};

use crate::{
	config::{
		self, ns,
		service::{ServiceKind, ServiceRouter},
	},
	context::{ProjectContext, RunContext, ServiceContext},
	dep::terraform::{self, output::read_k8s_cluster_aws},
};

// Kubernetes requires a specific port for containers because they have their own networking namespace, the
// port bound on the host is randomly generated.
pub const HEALTH_PORT: usize = 8000;
pub const METRICS_PORT: usize = 8001;
pub const TOKIO_CONSOLE_PORT: usize = 8002;
pub const HTTP_SERVER_PORT: usize = 80;

pub struct ExecServiceContext {
	pub svc_ctx: ServiceContext,
	pub run_context: RunContext,
	pub driver: ExecServiceDriver,
}

pub enum ExecServiceDriver {
	/// Used when building an uploading an image to docker.
	Docker { image_tag: String, force_pull: bool },
	/// Used when running a build binary locally.
	LocalBinaryArtifact {
		/// Path to the executable relative to the project root.
		exec_path: PathBuf,
		// TODO: Remove?
		args: Vec<String>,
	},
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SpecType {
	Deployment,
	Job,
	CronJob,
}

pub async fn project(ctx: &ProjectContext) -> Result<()> {
	// Check if the k8s provider has already applied
	let plan_id = match ctx.ns().kubernetes.provider {
		ns::KubernetesProvider::K3d { .. } => "k8s_cluster_k3d",
		ns::KubernetesProvider::AwsEks { .. } => "k8s_cluster_aws",
	};

	// TODO: has_applied is slow
	if !std::env::var("BOLT_SKIP_K8S_GEN")
		.ok()
		.map_or(false, |x| x == "1")
		&& terraform::cli::has_applied(ctx, plan_id).await
	{
		// Read kubectl config
		let config = match ctx.ns().kubernetes.provider {
			ns::KubernetesProvider::K3d { .. } => block_in_place(move || {
				cmd!("k3d", "kubeconfig", "get", ctx.k8s_cluster_name()).read()
			})?,
			ns::KubernetesProvider::AwsEks {} => {
				let output = read_k8s_cluster_aws(ctx).await;
				block_in_place(move || {
					cmd!(
						"aws",
						"eks",
						"update-kubeconfig",
						// Writes to stdout instead of user's kubeconfig
						"--dry-run",
						"--region",
						"us-east-1",
						"--name",
						ctx.k8s_cluster_name(),
						// Read from a non-existent kubeconfig to prevent it from merging the existing
						// user's kubeconfig.
						"--kubeconfig",
						"THIS DOES NOT EXIST",
						"--role-arn",
						output.eks_admin_role_arn.as_str(),
					)
					.read()
				})?
			}
		};

		// Write to file
		fs::create_dir_all(ctx.gen_kubeconfig_path().parent().unwrap()).await?;
		fs::write(ctx.gen_kubeconfig_path(), config).await?;
	} else {
		// Remove config
		if fs::try_exists(ctx.gen_kubeconfig_path()).await? {
			fs::remove_file(ctx.gen_kubeconfig_path()).await?;
		}
	}

	Ok(())
}

fn k8s_svc_name(exec_ctx: &ExecServiceContext) -> String {
	match &exec_ctx.run_context {
		RunContext::Service { .. } => format!("rivet-{}", exec_ctx.svc_ctx.name()),
		RunContext::Test { .. } => unreachable!(),
	}
}

/// Generates a job definition for services for the development Kubernetes cluster.
pub async fn gen_svc(exec_ctx: &ExecServiceContext) -> Vec<serde_json::Value> {
	let service_name = k8s_svc_name(exec_ctx);

	let ExecServiceContext {
		svc_ctx,
		run_context,
		driver,
	} = exec_ctx;

	let project_ctx = svc_ctx.project().await;
	let mut specs = Vec::new();

	let spec_type = match run_context {
		RunContext::Service { .. } => match svc_ctx.config().kind {
			ServiceKind::Headless { .. }
			| ServiceKind::Consumer { .. }
			| ServiceKind::Api { .. }
			| ServiceKind::Static { .. } => SpecType::Deployment,
			ServiceKind::Oneshot { .. } => SpecType::Job,
			ServiceKind::Periodic { .. } => SpecType::CronJob,
			ServiceKind::Operation { .. }
			| ServiceKind::Operations { .. }
			| ServiceKind::Database { .. }
			| ServiceKind::Cache { .. }
			| ServiceKind::ApiRoutes { .. } => {
				unreachable!()
			}
		},
		RunContext::Test { .. } => unreachable!(),
	};

	let has_health = matches!(run_context, RunContext::Service { .. })
		&& project_ctx
			.ns()
			.kubernetes
			.health_checks
			.unwrap_or_else(|| match project_ctx.ns().cluster.kind {
				config::ns::ClusterKind::SingleNode { .. } => false,
				config::ns::ClusterKind::Distributed { .. } => true,
			}) && matches!(
		svc_ctx.config().kind,
		ServiceKind::Headless { .. } | ServiceKind::Consumer { .. } | ServiceKind::Api { .. }
	);

	let has_metrics = matches!(
		svc_ctx.config().kind,
		ServiceKind::Headless { .. } | ServiceKind::Consumer { .. } | ServiceKind::Api { .. }
	);

	// Render env
	let env = svc_ctx.env(run_context).await.unwrap();
	let secret_env = svc_ctx.secret_env(run_context).await.unwrap();
	let env = generate_k8s_variables()
		.into_iter()
		.chain(
			env.into_iter()
				.map(|(k, v)| json!({ "name": k, "value": v })),
		)
		.collect::<Vec<_>>();
	let env_checksum = {
		let bytes = serde_json::to_vec(&env).unwrap();
		hex::encode(Sha256::digest(&bytes))
	};

	// Create secret env vars
	let secret_env_name = format!("{}-secret-env", service_name);
	let secret_data = secret_env
		.into_iter()
		.map(|(k, v)| (k, base64::encode(v)))
		.collect::<HashMap<_, _>>();
	let secret_env_checksum = {
		let bytes = serde_json::to_vec(&secret_data).unwrap();
		hex::encode(Sha256::digest(&bytes))
	};
	specs.push(json!({
		"apiVersion": "v1",
		"kind": "Secret",
		"metadata": {
			"name": secret_env_name,
			"namespace": "rivet-service"
		},
		"data": secret_data
	}));

	// Render ports
	let (pod_ports, service_ports) = {
		let mut pod_ports = Vec::new();
		let mut service_ports = Vec::new();

		pod_ports.push(json!({
			"name": "health",
			"containerPort": HEALTH_PORT,
		}));
		service_ports.push(json!({
			"name": "health",
			"protocol": "TCP",
			"port": HEALTH_PORT,
			"targetPort": "health"
		}));

		if has_metrics {
			pod_ports.push(json!({
				"name": "metrics",
				"containerPort": METRICS_PORT,
			}));
			service_ports.push(json!({
				"name": "metrics",
				"protocol": "TCP",
				"port": METRICS_PORT,
				"targetPort": "metrics"
			}));
		};

		if svc_ctx.enable_tokio_console() {
			pod_ports.push(json!({
				"name": "tokio-console",
				"containerPort": TOKIO_CONSOLE_PORT,
			}));
			service_ports.push(json!({
				"name": "tokio-console",
				"protocol": "TCP",
				"port": TOKIO_CONSOLE_PORT,
				"targetPort": "tokio-console"
			}));
		}

		if svc_ctx.config().kind.has_server() {
			if let ServiceKind::Api {
				port: Some(port), ..
			} = svc_ctx.config().kind
			{
				pod_ports.push(json!({
					"name": "http",
					"containerPort": port,
					"hostPort": port
				}));
				service_ports.push(json!({
					"name": "http",
					"protocol": "TCP",
					"port": port,
					"targetPort": "http"
				}));
			} else {
				pod_ports.push(json!({
					"name": "http",
					"containerPort": HTTP_SERVER_PORT,
				}));
				service_ports.push(json!({
					"name": "http",
					"protocol": "TCP",
					"port": HTTP_SERVER_PORT,
					"targetPort": "http"
				}));
			}
		}

		(pod_ports, service_ports)
	};

	let (readieness_probe, liveness_probe) = if has_health {
		(
			// Check if the HTTP server is reachable
			json!({
				"exec": {
					// This is intentionally using the /health/liveness endpoint for a readiness probe because
					// this checks if the HTTP server is reachable
					//
					// We don't want to check the essential services since this will cause a
					// complete outage if the essential probe sporadically fails. This is also
					// more lightweight.
					"command": ["/usr/bin/health_check.sh", "/health/liveness"],
				},
				"initialDelaySeconds": 1,
				"periodSeconds": 5,
				"timeoutSeconds": 5
			}),
			// Check if the essential connections are working
			//
			// This will cause cascading failure if one of the essential connections are down, so
			// we restart very conservatively (8 * 15 seconds = 2 minutes of downtime) as a worst case fallback.
			json!({
				"exec": {
				"command": ["/usr/bin/health_check.sh", "/health/essential"],
				},
				"initialDelaySeconds": 5,
				"periodSeconds": 30,
				"timeoutSeconds": 15,
				"failureThreshold": 8
			}),
		)
	} else {
		(serde_json::Value::Null, serde_json::Value::Null)
	};

	let (image, image_pull_policy, exec) = match &driver {
		ExecServiceDriver::LocalBinaryArtifact { exec_path, args } => (
			"ghcr.io/rivet-gg/rivet-local-binary-artifact-runner:07e8de0",
			"IfNotPresent",
			format!(
				"{} {}",
				Path::new("/target").join(exec_path).display(),
				args.join(" ")
			),
		),
		ExecServiceDriver::Docker {
			image_tag,
			force_pull,
			..
		} => (
			image_tag.as_str(),
			if *force_pull {
				"Always"
			} else {
				"IfNotPresent"
			},
			format!("/usr/bin/{}", svc_ctx.cargo_name().unwrap()),
		),
	};
	let command = format!("/usr/bin/install_ca.sh && {exec}");

	// Create resource limits
	let ns_service_config = svc_ctx.ns_service_config().await;
	let resources = if project_ctx.limit_resources() {
		json!({
			"requests": {
				"memory": format!("{}Mi", ns_service_config.resources.memory),
				"cpu": format!("{}m", ns_service_config.resources.cpu),
			},
			"limits": {
				// Allow oversubscribing memory
				"memory": format!("{}Mi", ns_service_config.resources.memory),
				"cpu": format!("{}m", ns_service_config.resources.cpu),
			},
		})
	} else {
		json!(null)
	};

	let (volumes, volume_mounts) = build_volumes(&project_ctx, &exec_ctx).await;

	let metadata = json!({
		"name": service_name,
		"namespace": "rivet-service",
		"labels": {
			"app.kubernetes.io/name": service_name
		}
	});

	let restart_policy = if spec_type == SpecType::Deployment {
		"Always"
	} else {
		"OnFailure"
	};

	// Create pod template
	let termination_grace_period = match project_ctx.ns().cluster.kind {
		// TODO: Allow configuring this
		// Kill pod immediately since we need workers to terminate immediately for tests
		//
		// i.e. `bolt up mm-worker && bolt test mm-worker` would cause requests to go to the old
		// worker if the old worker doesn't terminate immediately.
		config::ns::ClusterKind::SingleNode { .. } => 0,
		config::ns::ClusterKind::Distributed { .. } => 30,
	};
	let pod_spec = json!({
		"priorityClassName": "service-priority",
		"restartPolicy": restart_policy,
		"terminationGracePeriodSeconds": termination_grace_period,
		"imagePullSecrets": [{
			"name": "docker-auth"
		}],
		"containers": [{
			"name": "service",
			"image": image,
			"imagePullPolicy": image_pull_policy,
			"command": ["/bin/sh"],
			"args": ["-c", command],
			"env": env,
			"envFrom": [{
				"secretRef": {
					"name": secret_env_name
				}
			}],
			"volumeMounts": volume_mounts,
			"ports": pod_ports,
			"readinessProbe": readieness_probe,
			"livenessProbe": liveness_probe,
			"resources": resources,
		}],
		"volumes": volumes
	});
	let pod_template = json!({
		"metadata": {
			"labels": {
				"app.kubernetes.io/name": service_name
			},
			"annotations": {
				"checksum/env": env_checksum,
				"checksum/secret-env": secret_env_checksum
			},
		},
		"spec": pod_spec,
	});

	match spec_type {
		SpecType::Deployment => {
			specs.push(json!({
				"apiVersion": "apps/v1",
				"kind": "Deployment",
				"metadata": metadata,
				"spec": {
					// TODO: Does specifying this cause issues with HAP?
					"replicas": ns_service_config.count,
					"selector": {
						"matchLabels": {
							"app.kubernetes.io/name": service_name
						}
					},
					"template": pod_template
				}
			}));
		}
		SpecType::Job => {
			specs.push(json!({
				"apiVersion": "batch/v1",
				"kind": "Job",
				"metadata": metadata,
				"spec": {
					"completions": 1,
					// Keep retrying until completion
					"backoffLimit": 0,
					// TODO: Needed?
					// "parallelism": ns_service_config.count,
					"template": pod_template
				},
			}));
		}
		SpecType::CronJob => {
			let ServiceKind::Periodic {
				cron,
				prohibit_overlap,
				time_zone: _,
			} = &svc_ctx.config().kind
			else {
				unreachable!()
			};

			specs.push(json!({
				"apiVersion": "batch/v1",
				"kind": "CronJob",
				"metadata": metadata,
				"spec": {
					"schedule": cron,
					// Timezones are in alpha
					// "timeZone": time_zone,
					"concurrencyPolicy": if *prohibit_overlap {
						"Forbid"
					} else {
						"Allow"
					},
					"jobTemplate": {
						"spec": {
							"template": pod_template
						}
					}
				}
			}));
		}
	}

	// Horizontal Pod Autoscaler
	if matches!(
		project_ctx.ns().cluster.kind,
		config::ns::ClusterKind::Distributed { .. }
	) && matches!(
		svc_ctx.config().kind,
		ServiceKind::Headless {
			singleton: false,
			..
		} | ServiceKind::Consumer { .. }
			| ServiceKind::Api {
				singleton: false,
				..
			}
	) {
		specs.push(json!({
			"apiVersion": "autoscaling/v2",
			"kind": "HorizontalPodAutoscaler",
			"metadata": {
				"name": service_name,
				"namespace": "rivet-service",
				"labels": {
					"app.kubernetes.io/name": service_name
				}
			},
			"spec": {
				"scaleTargetRef": {
					"apiVersion": "apps/v1",
					"kind": "Deployment",
					"name": service_name
				},
				"minReplicas": ns_service_config.count,
				"maxReplicas": 5,
				"metrics": [
					{
						"type": "Resource",
						"resource": {
							"name": "cpu",
							"target": {
								"type": "Utilization",
								// Keep this low because the server is IO bound
								"averageUtilization": 50
							}
						}
					},
					{
						"type": "Resource",
						"resource": {
							"name": "memory",
							"target": {
								"type": "Utilization",
								"averageUtilization": 75
							}
						}
					},
				]
			}
		}));
	}

	// Expose service
	if matches!(run_context, RunContext::Service { .. }) {
		// Create service
		specs.push(json!({
			"apiVersion": "v1",
			"kind": "Service",
			"metadata": {
				"name": service_name,
				"namespace": "rivet-service",
				"labels": {
					"app.kubernetes.io/name": service_name
				}
			},
			"spec": {
				"type": "ClusterIP",
				"selector": {
					"app.kubernetes.io/name": service_name
				},
				"ports": service_ports
			}
		}));

		// Monitor the service
		if project_ctx.ns().prometheus.is_some() {
			specs.push(json!({
				"apiVersion": "monitoring.coreos.com/v1",
				"kind": "ServiceMonitor",
				"metadata": {
					"name": service_name,
					"namespace": "rivet-service"
				},
				"spec": {
					"selector": {
						"matchLabels": {
							"app.kubernetes.io/name": service_name
						},
					},
					"endpoints": [
						{ "port": "metrics" }
					],
				}
			}));
		}

		// Build ingress router
		if matches!(run_context, RunContext::Service { .. }) {
			if let Some(router) = svc_ctx.config().kind.router() {
				build_ingress_router(&project_ctx, svc_ctx, &service_name, &router, &mut specs);
			}
		}
	}

	specs
}

async fn build_volumes(
	project_ctx: &ProjectContext,
	exec_ctx: &ExecServiceContext,
) -> (Vec<serde_json::Value>, Vec<serde_json::Value>) {
	let ExecServiceContext {
		svc_ctx,
		run_context,
		driver,
	} = exec_ctx;

	// Shared data between containers
	let mut volumes = Vec::<serde_json::Value>::new();
	let mut volume_mounts = Vec::<serde_json::Value>::new();

	// Add volumes based on exec service
	match driver {
		// Mount the service binaries to execute directly in the container.
		ExecServiceDriver::LocalBinaryArtifact { .. } => {
			// Volumes
			volumes.push(json!({
				"name": "target",
				"hostPath": {
					"path": "/target",
					"type": "Directory"
				}
			}));
			volumes.push(json!({
				"name": "nix-store",
				"hostPath": {
					"path": "/nix/store",
					"type": "Directory"
				}
			}));

			// Mounts
			volume_mounts.push(json!({
				"name": "target",
				"mountPath": "/target",
				"readOnly": true
			}));
			volume_mounts.push(json!({
				"name": "nix-store",
				"mountPath": "/nix/store",
				"readOnly": true
			}));
		}
		ExecServiceDriver::Docker { .. } => {}
	}

	// Add Redis CA
	match project_ctx.ns().redis.provider {
		config::ns::RedisProvider::Kubernetes {} => {
			let redis_deps = svc_ctx
				.redis_dependencies(run_context)
				.await
				.iter()
				.map(|redis_dep| {
					if let config::service::RuntimeKind::Redis { persistent } =
						redis_dep.config().runtime
					{
						if persistent {
							"persistent"
						} else {
							"ephemeral"
						}
					} else {
						unreachable!();
					}
				})
				// IndexSet to avoid duplicates and keep order
				.collect::<IndexSet<_>>();

			volumes.extend(redis_deps.iter().map(|db| {
				json!({
					"name": format!("redis-{}-ca", db),
					"configMap": {
						"name": format!("redis-{}-ca", db),
						"defaultMode": 420,
						"items": [
							{
								"key": "ca.crt",
								"path": format!("redis-{}-ca.crt", db)
							}
						]
					}
				})
			}));
			volume_mounts.extend(redis_deps.iter().map(|db| {
				json!({
					"name": format!("redis-{}-ca", db),
					"mountPath": format!("/usr/local/share/ca-certificates/redis-{}-ca.crt", db),
					"subPath": format!("redis-{}-ca.crt", db)
				})
			}));
		}
		config::ns::RedisProvider::Aws { .. } | config::ns::RedisProvider::Aiven { .. } => {
			// Uses publicly signed cert
		}
	}

	// Add CRDB CA
	match project_ctx.ns().cockroachdb.provider {
		config::ns::CockroachDBProvider::Kubernetes {} => {
			volumes.push(json!({
				"name": "crdb-ca",
				"configMap": {
					"name": "crdb-ca",
					"defaultMode": 420,
					"items": [
						{
							"key": "ca.crt",
							"path": "crdb-ca.crt"
						}
					]
				}
			}));
			volume_mounts.push(json!({
				"name": "crdb-ca",
				"mountPath": "/usr/local/share/ca-certificates/crdb-ca.crt",
				"subPath": "crdb-ca.crt"
			}));
		}
		config::ns::CockroachDBProvider::Managed { .. } => {
			// Uses publicly signed cert
		}
	}

	// Add ClickHouse CA
	if let Some(clickhouse) = &project_ctx.ns().clickhouse {
		match &clickhouse.provider {
			config::ns::ClickHouseProvider::Kubernetes {} => {
				volumes.push(json!({
					"name": "clickhouse-ca",
					"configMap": {
						"name": "clickhouse-ca",
						"defaultMode": 420,
						"items": [
							{
								"key": "ca.crt",
								"path": "clickhouse-ca.crt"
							}
						]
					}
				}));
				volume_mounts.push(json!({
					"name": "clickhouse-ca",
					"mountPath": "/usr/local/share/ca-certificates/clickhouse-ca.crt",
					"subPath": "clickhouse-ca.crt"
				}));
			}
			// Uses publicly signed cert
			config::ns::ClickHouseProvider::Managed { .. } => {}
		}
	}

	(volumes, volume_mounts)
}

// Added for ease of use
pub fn generate_k8s_variables() -> Vec<serde_json::Value> {
	vec![
		json!({
			"name": "KUBERNETES_NODE_NAME",
			"valueFrom": {
				"fieldRef": {
					"fieldPath": "spec.nodeName"
				}
			},
		}),
		json!({
			"name": "KUBERNETES_POD_NAME",
			"valueFrom": {
				"fieldRef": {
					"fieldPath": "metadata.name"
				}
			}
		}),
		json!({
			"name": "KUBERNETES_POD_ID",
			"valueFrom": {
				"fieldRef": {
					"fieldPath": "metadata.uid"
				}
			}
		}),
	]
}

fn build_ingress_router(
	project_ctx: &ProjectContext,
	svc_ctx: &ServiceContext,
	service_name: &str,
	router: &ServiceRouter,
	specs: &mut Vec<serde_json::Value>,
) {
	// Filter mounts
	let enable_deprecated_subdomains = project_ctx
		.ns()
		.dns
		.as_ref()
		.map_or(false, |y| y.deprecated_subdomains);
	let mounts = router
		.mounts
		.iter()
		.filter(|x| !x.deprecated || enable_deprecated_subdomains);

	// Register all mounts with Traefik
	// TODO: move this in to a single ingressroute crd for web and websecure
	for (i, mount) in mounts.enumerate() {
		let mut rule = String::new();
		let mut middlewares = Vec::new();

		// Build host for rule
		if let (Some(domain_main), Some(domain_main_api)) =
			(project_ctx.domain_main(), project_ctx.domain_main_api())
		{
			// Derive routing info
			let domain = if let Some(subdomain) = &mount.subdomain {
				format!("{subdomain}.{domain_main}")
			} else {
				domain_main_api
			};

			rule.push_str(&format!("Host(`{domain}`)"));
		}

		if !rule.is_empty() {
			rule.push_str(" && ");
		}

		// Build paths for rule
		rule.push('(');
		rule.push_str(
			&mount
				.paths
				.iter()
				.map(|path| format!("PathPrefix(`{path}`)"))
				.collect::<Vec<_>>()
				.join(" || "),
		);
		rule.push(')');

		if let Some(strip_prefix) = &mount.strip_prefix {
			let mw_name = format!("{}-{i}-strip-prefix", svc_ctx.name());
			middlewares.push(json!({
				"apiVersion": "traefik.io/v1alpha1",
				"kind": "Middleware",
				"metadata": {
					"name": mw_name,
					"namespace": "rivet-service",
					"labels": {
						"traefik-instance": "main"
					}
				},
				"spec": {
					"stripPrefix": {
						"prefixes": [ strip_prefix ],
						"forceSlash": true
					}
				}
			}));
		}

		if let Some(add_path) = &mount.add_path {
			let mw_name = format!("{}-{i}-add-prefix", svc_ctx.name());
			middlewares.push(json!({
				"apiVersion": "traefik.io/v1alpha1",
				"kind": "Middleware",
				"metadata": {
					"name": mw_name,
					"namespace": "rivet-service",
					"labels": {
						"traefik-instance": "main"
					}
				},
				"spec": {
					"addPrefix": {
						"prefix": add_path,
					}
				}
			}));
		}

		// Compress
		{
			let mw_name = format!("{}-{i}-compress", svc_ctx.name());
			middlewares.push(json!({
				"apiVersion": "traefik.io/v1alpha1",
				"kind": "Middleware",
				"metadata": {
					"name": mw_name,
					"namespace": "rivet-service",
					"labels": {
						"traefik-instance": "main"
					}
				},
				"spec": {
					"compress": {}
				}
			}));
		}

		// In flight
		{
			let ip_header = if let Some(provider) = &project_ctx
				.ns()
				.dns
				.as_ref()
				.and_then(|dns| dns.provider.as_ref())
			{
				match provider {
					config::ns::DnsProvider::Cloudflare { .. } => "cf-connecting-ip",
				}
			} else {
				"x-forwarded-for"
			};

			let mw_name = format!("{}-{i}-inflight", svc_ctx.name());
			middlewares.push(json!({
				"apiVersion": "traefik.io/v1alpha1",
				"kind": "Middleware",
				"metadata": {
					"name": mw_name,
					"namespace": "rivet-service",
					"labels": {
						"traefik-instance": "main"
					}
				},
				"spec": {
					"inFlightReq": {
						"amount": 64,
						"sourceCriterion": {
							"requestHeaderName": ip_header,
						}
					}
				}
			}));
		}

		let ingress_middlewares = middlewares
			.iter()
			.map(
				|mw| json!({ "name": mw["metadata"]["name"], "namespace": mw["metadata"]["namespace"] }),
			)
			.collect::<Vec<_>>();

		specs.extend(middlewares);

		let priority = 50;

		// Build insecure router
		specs.push(json!({
			"apiVersion": "traefik.io/v1alpha1",
			"kind": "IngressRoute",
			"metadata": {
				"name": format!("{}-{i}-insecure", svc_ctx.name()),
				"namespace": "rivet-service",
				"labels": {
					"traefik-instance": "main"
				}
			},
			"spec": {
				"entryPoints": [ "web" ],
				"routes": [
					{
						"kind": "Rule",
						"match": rule,
						"priority": priority,
						"middlewares": ingress_middlewares,
						"services": [
							{
								"kind": "Service",
								"name": service_name,
								"namespace": "rivet-service",
								"port": "http"
							}
						]
					}
				],
			}
		}));

		// Build secure router
		if project_ctx.tls_enabled() {
			specs.push(json!({
				"apiVersion": "traefik.io/v1alpha1",
				"kind": "IngressRoute",
				"metadata": {
					"name": format!("{}-{i}-secure", svc_ctx.name()),
					"namespace": "rivet-service",
					"labels": {
						"traefik-instance": "main"
					}
				},
				"spec": {
					"entryPoints": [ "websecure" ],
					"routes": [
						{
							"kind": "Rule",
							"match": rule,
							"priority": priority,
							"middlewares": ingress_middlewares,
							"services": [
								{
									"kind": "Service",
									"name": service_name,
									"namespace": "rivet-service",
									"port": "http"
								}
							]
						}
					],
					"tls": {
						"secretName": "ingress-tls-cloudflare-cert",
						"options": {
							"name": "ingress-cloudflare",
							"namespace": "traefik"
						},
					}
				}
			}));
		}

		// Add CF challenge routes
		if svc_ctx.name() == "api-monolith" {
			specs.push(json!({
				"apiVersion": "traefik.io/v1alpha1",
				"kind": "IngressRoute",
				"metadata": {
					"name": "cf-verification-challenge-insecure",
					"namespace": "rivet-service",
					"labels": {
						"traefik-instance": "main"
					}
				},
				"spec": {
					"entryPoints": [ "web" ],
					"routes": [
						{
							"kind": "Rule",
							"match": "PathPrefix(`/.well-known/cf-custom-hostname-challenge/`)",
							"priority": 90,
							"services": [
								{
									"kind": "Service",
									"name": service_name,
									"namespace": "rivet-service",
									"port": "http"
								}
							]
						}
					]
				}
			}));

			if project_ctx.tls_enabled() {
				specs.push(json!({
					"apiVersion": "traefik.io/v1alpha1",
					"kind": "IngressRoute",
					"metadata": {
						"name": "cf-verification-challenge-secure",
						"namespace": "rivet-service",
						"labels": {
							"traefik-instance": "main"
						}
					},
					"spec": {
						"entryPoints": [ "websecure" ],
						"routes": [
							{
								"kind": "Rule",
								"match": "PathPrefix(`/.well-known/cf-custom-hostname-challenge/`)",
								"priority": 90,
								"services": [
									{
										"kind": "Service",
										"name": service_name,
										"namespace": "rivet-service",
										"port": "http"
									}
								]
							}
						],
						"tls": {
							"secretName": "ingress-tls-cloudflare-cert",
							"options": {
								"name": "ingress-cloudflare",
								"namespace": "traefik"
							},
						}
					}
				}));
			}
		}
	}
}
