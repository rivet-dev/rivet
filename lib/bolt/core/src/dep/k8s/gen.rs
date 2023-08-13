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
pub async fn gen_svc(region_id: &str, exec_ctx: &ExecServiceContext) -> serde_json::Value {
	let ExecServiceContext {
		svc_ctx,
		build_context: _,
		driver,
	} = exec_ctx;

	let project_ctx = svc_ctx.project().await;

	let service_name = format!("rivet-{}", svc_ctx.name());
	let service_tags = vec![
		"rivet".into(),
		svc_ctx.name(),
		format!("service-{}", svc_ctx.config().kind.short()),
		format!("runtime-{}", svc_ctx.config().runtime.short()),
	];

	// TODO: service vs oneshot/periodic

	// Render env
	let (env, forward_services) = svc_ctx.env(RunContext::Service).await.unwrap();
	assert!(
		forward_services.is_empty(),
		"should not forward services for RunContext::Service"
	);

	// TODO: `resources` block
	let ns_service_config = svc_ctx.ns_service_config().await;

	// TODO: app.kubernetes.io/managed-by
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
									"env": env
										.into_iter()
										.map(|(k, v)| json!({ "name": k, "value": v }))
										.collect::<Vec<_>>(),
									"volumeMounts": match driver {
										ExecServiceDriver::Docker { .. }
										| ExecServiceDriver::UploadedBinaryArtifact { .. } => None,
										// Mount the service binaries to execute directly in directly in the container. See
										// notes in salt/salt/nomad/files/nomad.d/client.hcl.j2.
										ExecServiceDriver::LocalBinaryArtifact { .. } => Some(json!([
											{
												"name": "backend-repo",
												"mountPath": "/var/rivet/backend",
												"readOnly": true
											},
											{
												"name": "nix-store",
												"mountPath": "/nix/store",
												"readOnly": true
											}
										])),
									}
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
					"volumes": match driver {
						ExecServiceDriver::Docker { .. }
						| ExecServiceDriver::UploadedBinaryArtifact { .. } => None,
		
						// Mount the service binaries to execute directly in directly in the container. See
						// notes in salt/salt/nomad/files/nomad.d/client.hcl.j2.
						ExecServiceDriver::LocalBinaryArtifact { .. } => Some([
							json!({
								"name": "backend-repo",
								"hostPath": {
									"path": "/var/rivet/backend",
									"type": "Directory"
								}
							}),
							json!({
								"name": "nix-store",
								"hostPath": {
									"path": "/nix/store",
									"type": "Directory"
								}
							})
						]),
					}
				}
			}
		}
	})
}
