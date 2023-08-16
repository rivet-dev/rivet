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
const HEALTH_PORT: usize = 1000;
const METRICS_PORT: usize = 1001;
const TOKIO_CONSOLE_PORT: usize = 1002;
const HTTP_SERVER_PORT: usize = 1003;

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
		// Mount the service binaries to execute directly in directly in the container. See
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

	// TODO: app.kubernetes.io/managed-by
	specs.insert(
		"deployment",
		json!({
			"apiVersion": "apps/v1",
			"kind": "Deployment",
			"metadata": {
				"name": service_name
			},
			"spec": {
				"replicas": 1,
				"selector": {
					"matchLabels": {
						"app": service_name
					}
				},
				"template": {
					"metadata": {
						"labels": {
							"app": service_name
						}
					},
					"spec": {
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
		json!({
			"name": "KUBERNETES_PORT_health",
			"value": HEALTH_PORT.to_string()
		}),
		json!({
			"name": "KUBERNETES_PORT_metrics",
			"value": METRICS_PORT.to_string()
		}),
		json!({
			"name": "KUBERNETES_PORT_tokio-console",
			"value": TOKIO_CONSOLE_PORT.to_string()
		}),
	]
}
