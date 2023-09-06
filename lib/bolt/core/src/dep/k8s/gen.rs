use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde_json::json;

use crate::{
	config::{
		self,
		ns::{LoggingProvider, S3Provider},
		service::{ServiceDomain, ServiceKind, ServiceRouter},
	},
	context::{BuildContext, ProjectContext, RunContext, ServiceContext},
};

// Kubernetes requires a specific port for containers because they have their own networking namespace, the
// port bound on the host is randomly generated.
pub const HEALTH_PORT: usize = 1000;
pub const METRICS_PORT: usize = 1001;
pub const TOKIO_CONSOLE_PORT: usize = 1002;
pub const HTTP_SERVER_PORT: usize = 1003;

pub struct ExecServiceContext {
	pub svc_ctx: ServiceContext,
	pub build_context: BuildContext,
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

/// Generates a job definition for services for the development Nomad cluster.
pub async fn gen_svc(
	region_id: &str,
	exec_ctx: &ExecServiceContext,
) -> HashMap<&'static str, serde_json::Value> {
	let ExecServiceContext {
		svc_ctx,
		build_context: _,
		driver,
	} = exec_ctx;

	let project_ctx = svc_ctx.project().await;
	let mut specs = HashMap::with_capacity(1);

	let (spec_type, _reschedule_attempts) = match svc_ctx.config().kind {
		ServiceKind::Headless { .. }
		| ServiceKind::Consumer { .. }
		| ServiceKind::Api { .. }
		| ServiceKind::Static { .. } => ("deployment", 0),
		ServiceKind::Oneshot { .. } => ("job", 1),
		ServiceKind::Periodic { .. } => ("cron-job", 1),
		ServiceKind::Operation { .. }
		| ServiceKind::Database { .. }
		| ServiceKind::Cache { .. } => {
			unreachable!()
		}
	};

	let service_name = format!("rivet-{}", svc_ctx.name());

	let has_health = project_ctx
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
	let (env, forward_services) = svc_ctx.env(RunContext::Service).await.unwrap();
	let (secret_env, secret_forward_services) =
		svc_ctx.secret_env(RunContext::Service).await.unwrap();
	assert!(
		forward_services.is_empty() && secret_forward_services.is_empty(),
		"should not forward services for RunContext::Service"
	);
	let env = generate_k8s_variables()
		.into_iter()
		.chain(
			env.into_iter()
				.map(|(k, v)| json!({ "name": k, "value": v })),
		)
		.collect::<Vec<_>>();

	// Render ports
	let ports = {
		let mut ports = Vec::new();

		ports.push(json!({
			"name": "health",
			"containerPort": HEALTH_PORT,
		}));

		if has_metrics {
			ports.push(json!({
				"name": "metrics",
				"containerPort": METRICS_PORT,
			}));
		};

		if svc_ctx.enable_tokio_console() {
			ports.push(json!({
				"name": "tokio-console",
				"containerPort": TOKIO_CONSOLE_PORT,
			}));
		}

		if svc_ctx.config().kind.has_server() {
			if let ServiceKind::Api {
				port: Some(port), ..
			} = svc_ctx.config().kind
			{
				ports.push(json!({
					"name": "http",
					"containerPort": port,
					"hostPort": port
				}));
			} else {
				ports.push(json!({
					"name": "http",
					"containerPort": HTTP_SERVER_PORT,
				}));
			}
		}

		ports
	};

	let health_check = if has_health {
		let cmd = if matches!(svc_ctx.config().kind, ServiceKind::Static { .. }) {
			"/local/health-checks.sh --static"
		} else {
			"/local/health-checks.sh"
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

	let (image, image_pull_policy, command, args) = match &driver {
		ExecServiceDriver::LocalBinaryArtifact { exec_path, args } => (
			"alpine:3.8",
			"IfNotPresent",
			vec![Path::new("/var/rivet/backend").join(exec_path)],
			args.clone(),
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
			Vec::new(),
			Vec::new(),
		),
	};

	// Create resource limits
	let ns_service_config = svc_ctx.ns_service_config().await;
	let resources = json!({
		// TODO: Disabled for now
		// "limits": {
		// 	"memory": format!("{}Mi", ns_service_config.resources.memory),
		// 	"cpu": match ns_service_config.resources.cpu {
		// 		config::ns::CpuResources::Cpu(x) => format!("{x}m"),
		// 		config::ns::CpuResources::CpuCores(x) => format!("{}m", x * 1000),
		// 	},
		// 	"ephemeral-storage": format!("{}M", ns_service_config.resources.ephemeral_disk)
		// },
		// "requests": {}
	});

	// Shared data between containers
	let mut volumes = vec![];
	let mut local = vec![json!({
		"configMap": {
			"name": "health-checks"
		}
	})];
	let mut volume_mounts = vec![json!({
		"name": "local",
		"mountPath": "/local",
		"readOnly": true
	})];

	// Add volumes
	{
		match driver {
			// Mount the service binaries to execute directly in the container. See
			// notes in salt/salt/nomad/files/nomad.d/client.hcl.j2.
			ExecServiceDriver::LocalBinaryArtifact { .. } => {
				volumes.push(json!({
					"name": "backend-repo",
					"hostPath": {
						"path": project_ctx.path(),
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
				volume_mounts.push(json!({
					"name": "backend-repo",
					"mountPath": "/var/rivet/backend",
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

		if svc_ctx.depends_on_region_config() {
			local.push(json!({
				"configMap": {
					"name": "region-config",
				}
			}));
		}

		volumes.push(json!({
			"name": "local",
			"projected": {
				"defaultMode": 0o0777,
				"sources": local
			}
		}));
	}

	// Create secret env vars
	let secret_env_name = format!("{}-secret-env", service_name);
	let secret_data = secret_env
		.into_iter()
		.map(|(k, v)| (k, base64::encode(v)))
		.collect::<HashMap<_, _>>();
	specs.insert(
		"secret-env",
		json!({
			"apiVersion": "v1",
			"kind": "Secret",
			"metadata": {
				"name": secret_env_name,
				"namespace": "rivet-service"
			},
			"data": secret_data
		}),
	);

	// Create priority class
	let priority_class_name = format!("{}-priority", service_name);
	specs.insert(
		"priority",
		json!({
			"apiVersion": "scheduling.k8s.io/v1",
			"kind": "PriorityClass",
			"metadata": {
				"name": priority_class_name,
				"namespace": "rivet-service"
			},
			"value": svc_ctx.config().service.priority()
		}),
	);

	// Create pod template
	let pod_template = json!({
		"metadata": {
			"labels": {
				"app.kubernetes.io/name": service_name
			}
		},
		"spec": {
			"priorityClassName": priority_class_name,
			// TODO: `OnFailure` Doesn't work with deployments
			"restartPolicy": if spec_type == "deployment" {
				"Always"
			} else {
				"OnFailure"
			},
			"imagePullSecrets": [{
				"name": "docker-auth"
			}],
			"containers": [{
				"name": "service",
				"image": image,
				"imagePullPolicy": image_pull_policy,
				"command": command,
				// "command": ["/bin/sh", "-c", "printenv && sleep 1000"],
				"args": args,
				"env": env,
				"envFrom": [{
					"secretRef": {
						"name": secret_env_name
					}
				}],
				"volumeMounts": volume_mounts,
				"ports": ports,
				"livenessProbe": health_check,
				"resources": resources
			}],
			"volumes": volumes
		}
	});

	if spec_type == "deployment" {
		specs.insert(
			"deployment",
			json!({
				"apiVersion": "apps/v1",
				"kind": "Deployment",
				"metadata": {
					"name": service_name,
					"namespace": "rivet-service",
					"labels": {
						"app.kubernetes.io/name": service_name
					}
				},
				"spec": {
					"replicas": ns_service_config.count,
					"selector": {
						"matchLabels": {
							"app.kubernetes.io/name": service_name
						}
					},
					"template": pod_template
				}
			}),
		);
	} else if spec_type == "job" {
		specs.insert(
			"job",
			json!({
				"apiVersion": "batch/v1",
				"kind": "Job",
				"metadata": {
					"name": service_name,
					"namespace": "rivet-service",
					"labels": {
						"app.kubernetes.io/name": service_name
					}
				},
				"spec": {
					"completions": 1,
					// "parallelism": ns_service_config.count,
					// Deletes job after it is finished
					"ttlSecondsAfterFinished": 1,
					"template": pod_template
				}
			}),
		);
	} else if let ServiceKind::Periodic {
		cron,
		prohibit_overlap,
		time_zone: _,
	} = &svc_ctx.config().kind
	{
		specs.insert(
			"cron-job",
			json!({
				"apiVersion": "batch/v1",
				"kind": "CronJob",
				"metadata": {
					"name": service_name,
					"namespace": "rivet-service",
					"labels": {
						"app.kubernetes.io/name": service_name
					}
				},
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
			}),
		);
	} else {
		panic!("invalid `spec_type` value `{}`", spec_type);
	}

	if svc_ctx.config().kind.has_server() {
		specs.insert(
			"http-server",
			json!({
				"apiVersion": "v1",
				"kind": "Service",
				"metadata": {
					"name": service_name,
					"namespace": "rivet-service"
				},
				"spec": {
					"type": "ClusterIP",
					"selector": {
						"app.kubernetes.io/name": service_name
					},
					"ports": [{
						"protocol": "TCP",
						"port": 80,
						"targetPort": "http"
					}]
				}
			}),
		);
	}

	if let Some(router) = svc_ctx.config().kind.router() {
		build_ingress_router(&project_ctx, svc_ctx, &service_name, &router, &mut specs);
	}

	specs
}

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
	specs: &mut HashMap<&str, serde_json::Value>,
) {
	// Enable Traefik if there are mounts
	if !router.mounts.is_empty() {}

	// Register all mounts with Traefik
	for (i, mount) in router.mounts.iter().enumerate() {
		// Derive routing info
		let domain = match mount.domain {
			ServiceDomain::Base => project_ctx.domain_main(),
			ServiceDomain::BaseGame => project_ctx.domain_cdn(),
			ServiceDomain::BaseJob => project_ctx.domain_job(),
		};
		let host = if let Some(subdomain) = &mount.subdomain {
			format!("{}.{}", subdomain, domain)
		} else {
			domain
		};

		// Build rule
		let mut rule = format!("Host(`{host}`)");
		if let Some(path) = &mount.path {
			rule.push_str(&format!(" && PathPrefix(`{path}`)"));
		}

		// Build middlewares
		let mut middlewares = Vec::new();

		// Strip prefix
		if let Some(path) = &mount.path {
			let mw_name = format!("{}-{i}-strip-prefix", svc_ctx.name());
			middlewares.push(json!({
				"apiVersion": "traefik.containo.us/v1alpha1",
				"kind": "Middleware",
				"metadata": {
					"name": mw_name,
					"namespace": "traefik"
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
				"apiVersion": "traefik.containo.us/v1alpha1",
				"kind": "Middleware",
				"metadata": {
					"name": mw_name,
					"namespace": "traefik"
				},
				"spec": {
					"compress": {}
				}
			}));
		}

		// In flight
		{
			let mw_name = format!("{}-{i}-inflight", svc_ctx.name());
			middlewares.push(json!({
				"apiVersion": "traefik.containo.us/v1alpha1",
				"kind": "Middleware",
				"metadata": {
					"name": mw_name,
					"namespace": "traefik"
				},
				"spec": {
					"inFlightReq": {
						"amount": 64,
						"sourceCriterion": {
							"requestHeaderName": "cf-connecting-ip"
						}
					}
				}
			}));
		}

		let ingress_middlewares = middlewares
			.iter()
			.map(|mw| mw["metadata"].clone())
			.collect::<Vec<_>>();

		specs.insert(
			"middlewares",
			json!({
				"apiVersion": "v1",
				"kind": "List",
				"items": middlewares
			}),
		);

		// Build insecure router
		specs.insert(
			"ingress-insecure",
			json!({
				"apiVersion": "traefik.containo.us/v1alpha1",
				"kind": "IngressRoute",
				"metadata": {
					"name": format!("{}-{i}-insecure", svc_ctx.name()),
					"namespace": "traefik"
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
									"port": 80
								}
							]
						}
					],
				}
			}),
		);

		// Build secure router
		specs.insert(
			"ingress-secure",
			json!({
				"apiVersion": "traefik.containo.us/v1alpha1",
				"kind": "IngressRoute",
				"metadata": {
					"name": format!("{}-{i}-secure", svc_ctx.name()),
					"namespace": "traefik"
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
									"port": 80
								}
							]
						}
					],
					"tls": {
						"secretName": "ingress-tls-cert",
						"options": {
							"name": "ingress-tls",
							"namespace": "traefik"
						},
						// "domains": [
						// 	{
						// 		"main": "example.net",
						// 		"sans": [
						// 			"a.example.net",
						// 			"b.example.net"
						// 		]
						// 	}
						// ]
					}
				}
			}),
		);

		if svc_ctx.name() == "api-cf-verification" {
			specs.insert(
				"challenge-ingress",
				json!({
					"apiVersion": "traefik.containo.us/v1alpha1",
					"kind": "IngressRoute",
					"metadata": {
						"name": "cf-verification-challenge",
						"namespace": "traefik"
					},
					"spec": {
						"routes": [
							{
								"kind": "Rule",
								"match": "PathPrefix(`/.well-known/cf-custom-hostname-challenge/`)",
								"priority": 90
							}
						]
					}
				}),
			);
		}
	}
}
