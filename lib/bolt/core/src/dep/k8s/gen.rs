use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::Result;
use serde_json::json;

use crate::{
	config::{
		self,
		ns::LoggingProvider,
		service::{ServiceDomain, ServiceKind, ServiceRouter},
	},
	context::{BuildContext, ProjectContext, RunContext, S3Provider, ServiceContext},
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
		image: String,
		force_pull: bool,
	},
	UploadedBinaryArtifact {
		artifact_key: String,
		/// Path to the executable within the archive.
		exec_path: String,
		// TODO: Remove?
		args: Vec<String>,
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

	let service_name = format!("rivet-{}", svc_ctx.name());
	let service_tags = vec![
		"rivet".into(),
		svc_ctx.name(),
		format!("service-{}", svc_ctx.config().kind.short()),
		format!("runtime-{}", svc_ctx.config().runtime.short()),
	];

	let has_health = project_ctx.ns().nomad.health_checks.unwrap_or_else(|| {
		match project_ctx.ns().cluster.kind {
			config::ns::ClusterKind::SingleNode { .. } => false,
			config::ns::ClusterKind::Distributed { .. } => true,
		}
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
	assert!(
		forward_services.is_empty(),
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

	// TODO: `resources` block
	let ns_service_config = svc_ctx.ns_service_config().await;

	// Shared data between containers
	let mut volumes = vec![json!({
		"name": "shared-data",
		"emptyDir": {}
	})];
	let mut volume_mounts = vec![json!({
		"name": "shared-data",
		"mountPath": "/local/shared",
		"readOnly": true
	})];

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
		ExecServiceDriver::Docker { .. } | ExecServiceDriver::UploadedBinaryArtifact { .. } => {}
	}

	if svc_ctx.depends_on_region_config() {
		volumes.push(json!({
			"name": "region-config",
			"configMap": {
				"name": "region-config",
			}
		}));
		volume_mounts.push(json!({
			"name": "region-config",
			"mountPath": "/local",
			"readOnly": true
		}));
	}

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

	// TODO: app.kubernetes.io/managed-by
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
				"replicas": 1,
				"selector": {
					"matchLabels": {
						"app.kubernetes.io/name": service_name
					}
				},
				"template": {
					"metadata": {
						"labels": {
							"app.kubernetes.io/name": service_name
						}
					},
					"spec": {
						"priorityClassName": priority_class_name,
						"containers": [
							match &driver {
								ExecServiceDriver::Docker { image, force_pull } => {
									todo!()
									// json!({
									// 	"image": image,
									// 	"imagePullPolicy": if *force_pull { "Always" } else {
									// 		"IfNotPresent"
									// 	},
									// })
								}
								ExecServiceDriver::LocalBinaryArtifact { exec_path, args } => {
									json!({
										"name": "alpine",
										"image": "alpine:3.8",
										"command": [Path::new("/var/rivet/backend").join(exec_path)],
										// "command": ["/bin/sh", "-c", "printenv && sleep 1000"],
										"args": args,
										"env": env,
										"volumeMounts": volume_mounts,
										"ports": ports
									})
								}
								_ => todo!(),
								// TODO:
								// ExecServiceDriver::UploadedBinaryArtifact { exec_path, args, .. } => {
								// 	json!({
								// 		"image": "alpine:3.18",
								// 		"command": format!("${{NOMAD_TASK_DIR}}/build/{exec_path}"),
								// 		"args": args,
								// 		"auth": nomad_docker_io_auth(&project_ctx).await.unwrap(),
								// 		"logging": nomad_loki_plugin_config(&project_ctx, &svc_ctx).await.unwrap(),
								// 	})
								// }
								// ExecServiceDriver::Binary { path, args } => json!({}),
							}
						],
						"volumes": volumes
					}
				}
			}
		}),
	);

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
					"namespace": "rivet-service"
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
					"namespace": "rivet-service"
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
					"namespace": "rivet-service"
				},
				"spec": {
					"inFlightReq": {
						"amount": 64,
						"sourcecriterion": {
							"requestheadername": "cf-connecting-ip"
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

		// Build router
		let r_name = format!("{}-{i}", svc_ctx.name());

		specs.insert(
			"ingress",
			json!({
				"apiVersion": "traefik.containo.us/v1alpha1",
				"kind": "IngressRoute",
				"metadata": {
					"name": r_name,
					"namespace": "rivet-service"
				},
				"spec": {
					// "entryPoints": [ "lb-443" ],
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
							"namespace": "rivet-service"
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
						"namespace": "rivet-service"
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
