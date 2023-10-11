use std::{
	collections::HashMap,
	path::{Path, PathBuf},
};

use anyhow::Result;
use duct::cmd;
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
	Docker {
		image_tag: String,
		force_pull: bool,
	},
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
	if terraform::cli::has_applied(ctx, plan_id).await {
		// Read kubectl config
		let config = match ctx.ns().kubernetes.provider {
			ns::KubernetesProvider::K3d {} => block_in_place(move || {
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

pub fn k8s_svc_name(exec_ctx: &ExecServiceContext) -> String {
	match &exec_ctx.run_context {
		RunContext::Service { .. } => format!("rivet-{}", exec_ctx.svc_ctx.name()),
		RunContext::Test { test_id } => {
			format!("rivet-{}-test-{test_id}", exec_ctx.svc_ctx.name())
		}
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
			| ServiceKind::Database { .. }
			| ServiceKind::Cache { .. } => {
				unreachable!()
			}
		},
		RunContext::Test { .. } => SpecType::Job,
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

	let health_check = if has_health {
		let cmd = if matches!(svc_ctx.config().kind, ServiceKind::Static { .. }) {
			"/local/rivet/health-checks.sh --static"
		} else {
			"/local/rivet/health-checks.sh"
		};

		json!({
			"exec": {
				"command": ["/bin/sh", "-c", cmd],
			},
			"initialDelaySeconds": 1,
			"periodSeconds": 5,
			"timeoutSeconds": 5
		})
	} else {
		serde_json::Value::Null
	};

	let (image, image_pull_policy, exec) = match &driver {
		ExecServiceDriver::LocalBinaryArtifact { exec_path, args } => (
			"ghcr.io/rivet-gg/rivet-local-binary-artifact-runner:3fdc702",
			"IfNotPresent",
			format!(
				"{} {}",
				Path::new("/rivet-src").join(exec_path).display(),
				args.join(" ")
			),
		),
		ExecServiceDriver::Docker {
			image_tag,
			force_pull,
		} => (
			image_tag.as_str(),
			if *force_pull {
				"Always"
			} else {
				"IfNotPresent"
			},
			"bin/svc".to_string(),
		),
	};
	let command = format!("/local/rivet/install-ca.sh && {exec}");

	// Create resource limits
	let ns_service_config = svc_ctx.ns_service_config().await;
	let resources = if project_ctx.limit_resources() {
		json!({
			"requests": {
				"memory": format!("{}Mi", ns_service_config.resources.memory),
				"cpu": match ns_service_config.resources.cpu {
					config::ns::CpuResources::Cpu(x) => format!("{x}m"),
					config::ns::CpuResources::CpuCores(x) => format!("{}m", x * 1000),
				},
				"ephemeral-storage": format!("{}M", ns_service_config.resources.ephemeral_disk)
			},
			"limits": {
				"memory": format!("{}Mi", ns_service_config.resources.memory * 2),
				"cpu": match ns_service_config.resources.cpu {
					config::ns::CpuResources::Cpu(x) => format!("{}m", x * 2),
					config::ns::CpuResources::CpuCores(x) => format!("{}m", x * 1000 * 2),
				},
				"ephemeral-storage": format!("{}M", ns_service_config.resources.ephemeral_disk * 2)
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
		},
		"annotations": {
			"checksum/env": env_checksum,
			"checksum/secret-env": secret_env_checksum
		}
	});

	// Create priority class
	let priority_class_name = format!("{}-priority", service_name);
	specs.push(json!({
		"apiVersion": "scheduling.k8s.io/v1",
		"kind": "PriorityClass",
		"metadata": {
			"name": priority_class_name,
			"namespace": "rivet-service"
		},
		"value": svc_ctx.config().service.priority()
	}));

	let restart_policy = if matches!(run_context, RunContext::Test { .. }) {
		"Never"
	} else if spec_type == SpecType::Deployment {
		"Always"
	} else {
		"OnFailure"
	};

	// Create pod template
	let pod_spec = json!({
		"priorityClassName": priority_class_name,
		"restartPolicy": restart_policy,
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
			"livenessProbe": health_check,
			"resources": resources,
		}],
		"volumes": volumes
	});
	let pod_template = json!({
		"metadata": {
			"labels": {
				"app.kubernetes.io/name": service_name
			}
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
			let spec = if matches!(run_context, RunContext::Test { .. }) {
				json!({
					"ttlSecondsAfterFinished": 3,
					"completions": 1,
					"backoffLimit": 0,
					"template": pod_template
				})
			} else {
				json!({
					"completions": 1,
					// TODO: Needed?
					// "parallelism": ns_service_config.count,
					"template": pod_template
				})
			};

			specs.push(json!({
				"apiVersion": "batch/v1",
				"kind": "Job",
				"metadata": metadata,
				"spec": spec
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
	let mut volumes = vec![json!({
		"name": "local",
		"projected": {
			"defaultMode": 0o0777,
			"sources": [
				{
					"configMap": {
						"name": "health-checks"
					}
				},
				{
					"configMap": {
						"name": "install-ca"
					}
				}
			]
		}
	})];
	let mut volume_mounts = vec![json!({
		"name": "local",
		"mountPath": "/local/rivet",
		"readOnly": true
	})];

	// Add volumes based on exec service
	match driver {
		// Mount the service binaries to execute directly in the container.
		ExecServiceDriver::LocalBinaryArtifact { .. } => {
			// Volumes
			volumes.push(json!({
				"name": "rivet-src",
				"hostPath": {
					"path": "/rivet-src",
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
				"name": "rivet-src",
				"mountPath": "/rivet-src",
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

	// Add CA volumes
	if let config::ns::ClusterKind::SingleNode { .. } = &project_ctx.ns().cluster.kind {
		let redis_deps = svc_ctx
			.redis_dependencies(run_context)
			.await
			.iter()
			.map(|dep| dep.redis_db_name())
			.collect::<Vec<_>>();

		// Redis CA
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

		// CRDB CA
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

		// Clickhouse CA
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

	(volumes, volume_mounts)
}

// Added for ease of use
fn generate_k8s_variables() -> Vec<serde_json::Value> {
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
	// Register all mounts with Traefik
	for (i, mount) in router.mounts.iter().enumerate() {
		// Build host rule
		let mut rule = String::new();

		// Build host
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

		// Build path
		if let Some(path) = &mount.path {
			if !rule.is_empty() {
				rule.push_str(" && ");
			}

			rule.push_str(&format!("PathPrefix(`{path}`)"));
		}

		// Build middlewares
		let mut middlewares = Vec::new();

		// Strip prefix
		if let Some(path) = &mount.path {
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
						"prefixes": [ path ],
						"forceSlash": true
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

		// TODO: Add back
		// In flight
		// {
		// 	let mw_name = format!("{}-{i}-inflight", svc_ctx.name());
		// 	middlewares.push(json!({
		// 		"apiVersion": "traefik.io/v1alpha1",
		// 		"kind": "Middleware",
		// 		"metadata": {
		// 			"name": mw_name,
		// 			"namespace": "rivet-service"
		// 		},
		// 		"spec": {
		// 			"inFlightReq": {
		// 				"amount": 64,
		// 				"sourceCriterion": {
		// 					"requestHeaderName": "cf-connecting-ip"
		// 				}
		// 			}
		// 		}
		// 	}));
		// }

		let ingress_middlewares = middlewares
			.iter()
			.map(
				|mw| json!({ "name": mw["metadata"]["name"], "namespace": mw["metadata"]["namespace"] }),
			)
			.collect::<Vec<_>>();

		specs.extend(middlewares);

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

		if svc_ctx.name() == "api-cf-verification" {
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
