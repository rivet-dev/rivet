use anyhow::Result;
use serde_json::json;
use std::path::{Path, PathBuf};

use crate::{
	config::{
		self,
		service::{ServiceDomain, ServiceKind, ServiceRouter},
	},
	context::{BuildContext, ProjectContext, RunContext, ServiceContext},
	dep::{nomad::job_schema::*, s3},
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
		args: Vec<String>,
	},
	LocalBinaryArtifact {
		/// Path relative to the project root.
		path: PathBuf,
		args: Vec<String>,
	},
}

/// Generates a job definition for services for the development Nomad cluster.
pub async fn gen_svc(region_id: &str, exec_ctx: &ExecServiceContext) -> Job {
	let ExecServiceContext {
		svc_ctx,
		build_context: _,
		driver,
	} = exec_ctx;

	let project_ctx = svc_ctx.project().await;

	let node_class = match project_ctx.ns().deploy.kind {
		config::ns::DeployKind::Local { .. } => "local",
		config::ns::DeployKind::Cluster { .. } => "svc",
	};

	let (job_type, enable_update, reschedule_attempts) = match svc_ctx.config().kind {
		ServiceKind::Headless { .. }
		| ServiceKind::Consumer { .. }
		| ServiceKind::Api { .. }
		| ServiceKind::Static { .. } => ("service", true, 0),
		ServiceKind::Oneshot { .. } | ServiceKind::Periodic { .. } => ("batch", false, 1),
		ServiceKind::Operation { .. }
		| ServiceKind::Database { .. }
		| ServiceKind::Cache { .. } => {
			unreachable!()
		}
	};

	let service_name = format!("rivet-{}", svc_ctx.name());
	let service_tags = vec![
		"rivet".into(),
		svc_ctx.name(),
		format!("service-{}", svc_ctx.config().kind.short()),
		format!("runtime-{}", svc_ctx.config().runtime.short()),
	];

	let has_health = project_ctx.ns().nomad.health_checks.unwrap_or_else(|| {
		match project_ctx.ns().deploy.kind {
			config::ns::DeployKind::Local { .. } => false,
			config::ns::DeployKind::Cluster { .. } => true,
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

	// Render ports
	let (dynamic_ports, reserved_ports) = {
		let mut reserved_ports = vec![];
		let mut ports = Vec::new();

		ports.push(DynamicPort {
			label: Some("health".into()),
			to: Some(-1),
			..Default::default()
		});

		if has_metrics {
			ports.push(DynamicPort {
				label: Some("metrics".into()),
				to: Some(-1),
				..Default::default()
			});
		};

		if svc_ctx.enable_tokio_console() {
			ports.push(DynamicPort {
				label: Some("tokio-console".into()),
				to: Some(-1),
				..Default::default()
			});
		}

		if svc_ctx.config().kind.has_server() {
			if let ServiceKind::Api {
				port: Some(port), ..
			} = svc_ctx.config().kind
			{
				reserved_ports.push(json!({
					"Label": "http",
					"Value": port,
				}));
			} else {
				ports.push(DynamicPort {
					label: Some("http".into()),
					..Default::default()
				});
			}
		}

		(ports, reserved_ports)
	};
	let docker_ports = dynamic_ports
		.iter()
		.map(|x| x.label.clone().unwrap())
		.chain(
			reserved_ports
				.iter()
				.map(|x| x.get("Label").unwrap().as_str().unwrap().to_string()),
		)
		.collect::<Vec<String>>();

	let ns_service_config = svc_ctx.ns_service_config().await;

	Job {
		id: Some(format!("rivet-{}:{}", svc_ctx.name(), region_id)),
		job_type: Some(job_type.to_string()),
		priority: Some(svc_ctx.config().service.priority() as i64),
		region: Some("global".into()),
		datacenters: Some(vec![region_id.into()]),
		constraints: Some(json!([
			{
				"LTarget": "${node.class}",
				"RTarget": node_class,
				"Operand": "=",
			}
		])),
		periodic: if let ServiceKind::Periodic {
			cron,
			prohibit_overlap,
			time_zone,
		} = &svc_ctx.config().kind
		{
			Some(json!({
				"Enabled": true,
				"SpecType": "cron",
				"Spec": cron,
				"ProhibitOverlap": prohibit_overlap,
				"TimeZone": time_zone,
			}))
		} else {
			None
		},
		task_groups: Some(vec![TaskGroup {
			name: Some("svc".into()),
			// TODO: Add autoscaling
			count: Some(ns_service_config.count as i64),
			update: if enable_update {
				Some(
					// Wait for services to be healthy before progressing deploy
					if has_health {
						Update {
							max_parallel: Some(1),
							health_check: Some("checks".to_owned()),
							min_healthy_time: Some(chrono::secs(10)),
							healthy_deadline: Some(chrono::min(5)),
							progress_deadline: Some(chrono::min(10)),
							auto_revert: Some(true),
							canary: Some(0),
							stagger: Some(chrono::secs(30)),
							..Default::default()
						}
					} else {
						// Just monitor the task status
						Update {
							health_check: Some("task_states".into()),
							..Default::default()
						}
					},
				)
			} else {
				None
			},
			reschedule_policy: Some(ReschedulePolicy {
				delay: Some(chrono::secs(30)),
				delay_function: Some("constant".into()),
				attempts: Some(reschedule_attempts),
				unlimited: Some(reschedule_attempts == 0),
				..Default::default()
			}),
			networks: Some(vec![Network {
				mode: Some("bridge".into()),
				dynamic_ports: Some(dynamic_ports),
				reserved_ports: Some(json!(reserved_ports)),
				// TODO: This is hardcoded to the nomad host
				// Define Consul server address since running in an isoalted container.
				dns: Some(json!({
					 "Servers": ["172.26.64.1"],
				})),
				// dns: match driver {
				// 	// TODO: This is hardcoded to the nomad host
				// 	// Define Consul server address since running in an isoalted container.
				// 	ExecServiceDriver::Docker { .. }
				// 	| ExecServiceDriver::UploadedBinaryArtifact { .. } => Some(json!({
				// 		 "Servers": ["172.26.64.1"],
				// 	})),
				// 	// Uses the host's DNS.
				// 	ExecServiceDriver::LocalBinaryArtifact { .. } => None,
				// },
				..Default::default()
			}]),
			ephemeral_disk: Some(json!({
				// Prevent exhausting all storage resources on the local machine. We don't
				// accept anything over 512 MB since our dev machiens are limited in space.
				"SizeMB": ns_service_config.resources.ephemeral_disk,
			})),
			services: Some({
				let mut services = Vec::new();

				// Add default service
				services.push(Service {
					name: Some(service_name.clone()),
					port_label: if svc_ctx.config().kind.has_server() {
						Some("http".into())
					} else {
						None
					},
					// We use the host port, since we connect directly to this
					// service.
					address_mode: Some("host".into()),
					tags: Some({
						let mut tags = service_tags.clone();
						tags.push("http".into());
						if let Some(router) = svc_ctx.config().kind.router() {
							build_router_service_tags(&project_ctx, svc_ctx, &router, &mut tags);
						}
						tags
					}),
					checks: if has_health {
						let mut checks = Vec::new();

						// Require a healthy health check before booting
						let interval = chrono::secs(15);
						let on_update = "require_healthy";

						checks.push(Check {
							name: Some("Health Server Liveness".into()),
							check_type: Some("http".into()),
							port_label: Some("health".into()),
							path: Some("/health/liveness".into()),
							interval: Some(chrono::secs(15)),
							timeout: Some(chrono::secs(5)),
							on_update: Some(on_update.into()),
							// This indicates the runtime might
							// be bricked in a very rare care.
							// Restarting the process will fix
							// that.
							//
							// Ignore in staging since we have
							// `on_update` set to `ignore`.
							check_restart: Some(CheckRestart {
								limit: Some(3),
								..Default::default()
							}),
							..Default::default()
						});

						checks.push(build_conn_check(
							"Nats Connection",
							"/health/nats",
							interval,
							on_update,
						));

						checks.push(build_conn_check(
							"Redis Chirp Connection",
							"/health/redis/redis-chirp",
							interval,
							on_update,
						));

						if matches!(svc_ctx.config().kind, ServiceKind::Static { .. }) {
							checks.push(Check {
								name: Some("Static Root Accessible".into()),
								check_type: Some("http".into()),
								port_label: Some("http".into()),
								path: Some("/".into()),
								interval: Some(chrono::secs(10)),
								timeout: Some(chrono::secs(15)),
								..Default::default()
							});
						}

						Some(checks)
					} else {
						None
					},
					..Default::default()
				});

				// Add Consul Connect sidecar if needed
				if matches!(
					svc_ctx.config().kind,
					ServiceKind::Api {
						consul_connect: true,
						..
					}
				) {
					services.push(Service {
						name: Some(service_name.clone()),
						port_label: if svc_ctx.config().kind.has_server() {
							Some("http".into())
						} else {
							None
						},
						// We need to use the alloc port for tasks exposed with
						// Consul Connect
						address_mode: Some("alloc".into()),
						tags: Some(
							[service_tags.clone(), vec!["http-connect".to_owned()]].concat(),
						),
						connect: match svc_ctx.config().kind {
							ServiceKind::Api {
								consul_connect: true,
								..
							} => Some(Connect {
								sidecar_service: Some(SidecarService {
									proxy: Some(Proxy::default()),
									..Default::default()
								}),
								..Default::default()
							}),
							_ => None,
						},
						..Default::default()
					});
				}

				// Add metrics
				if has_metrics {
					services.push(Service {
						name: Some(service_name.clone()),
						port_label: Some("metrics".into()),
						address_mode: Some("host".into()),
						tags: Some([service_tags.clone(), vec!["metrics".to_owned()]].concat()),
						..Default::default()
					});
				}

				// Add Tokio Console
				if svc_ctx.enable_tokio_console() {
					services.push(Service {
						name: Some(service_name.clone()),
						port_label: Some("tokio-console".into()),
						address_mode: Some("host".into()),
						tags: Some(
							[service_tags.clone(), vec!["tokio-console".to_owned()]].concat(),
						),
						..Default::default()
					});
				}

				services
			}),
			tasks: Some(vec![Task {
				name: Some("svc".into()),
				restart_policy: Some(match svc_ctx.config().kind {
					// Prevent periodic jobs from retrying forever
					ServiceKind::Periodic { .. } => RestartPolicy {
						attempts: Some(2),
						delay: Some(chrono::secs(15)),
						interval: Some(chrono::min(5)),
						mode: Some("fail".to_owned()),
						..Default::default()
					},
					_ => RestartPolicy {
						attempts: Some(2),
						delay: Some(chrono::secs(15)),
						interval: Some(chrono::min(15)),
						mode: Some("fail".to_owned()),
						..Default::default()
					},
				}),
				driver: Some(match driver {
					ExecServiceDriver::Docker { .. } => "docker".into(),
					ExecServiceDriver::LocalBinaryArtifact { .. }
					| ExecServiceDriver::UploadedBinaryArtifact { .. } => "docker".into(),
				}),
				config: Some(match &driver {
					ExecServiceDriver::Docker { image, force_pull } => {
						// TODO: Allow configuring auth
						json!({
							"image": image,
							"force_pull": force_pull,
							"ports": docker_ports,
							"auth": nomad_docker_io_auth(&project_ctx).await.unwrap(),
						})
					}
					ExecServiceDriver::LocalBinaryArtifact { path, args } => {
						json!({
							"image": "alpine:3.18",
							"args": args,
							"command": Path::new("/var/rivet/backend").join(path),
							"auth": nomad_docker_io_auth(&project_ctx).await.unwrap(),
						})
					}
					ExecServiceDriver::UploadedBinaryArtifact { args, .. } => {
						json!({
							"image": "alpine:3.18",
							"command": format!("${{NOMAD_TASK_DIR}}/build/{}", svc_ctx.name()),
							"args": args,
							"auth": nomad_docker_io_auth(&project_ctx).await.unwrap(),
						})
					}
					// TODO: This doesn't work since we don't handle Ctrl-C
					// correctly still
					// ExecServiceDriver::Binary { path, args } => json!({
					// 	"command": "heaptrack",
					// 	"args": ({
					// 		let mut x = vec![
					// 			"-o".to_string(),
					// 			format!("/tmp/heaptrack-{}-${{NOMAD_ALLOC_ID}}", svc_ctx.name()),
					// 		];
					// 		x.push(path.display().to_string());
					// 		x.extend(args.clone());
					// 		x
					// 	}),
					// }),
				}),
				env: Some(env.into_iter().collect()),
				artifacts: match driver {
					ExecServiceDriver::Docker { .. }
					| ExecServiceDriver::LocalBinaryArtifact { .. } => None,
					ExecServiceDriver::UploadedBinaryArtifact { artifact_key, .. } => {
						let service_key = s3::fetch_service_key(
							&project_ctx,
							&["b2", "bolt_service_builds_download"],
						)
						.await
						.unwrap();
						Some(json!([
							{
								"GetterMode": "dir",
								"GetterSource": format!(
									"s3::{endpoint}/{bucket}/{key}",
									endpoint = s3::service_builds::ENDPOINT,
									bucket = s3::service_builds::BUCKET,
									key = urlencoding::encode(artifact_key),
								),
								"GetterOptions": {
									"region": s3::service_builds::REGION,
									"aws_access_key_id": service_key.key_id,
									"aws_access_key_secret": service_key.key,
									// "checksum": TODO,
								},
								"RelativeDest": "${NOMAD_TASK_DIR}/build/",
							}
						]))
					}
				},
				templates: Some({
					let mut templates = Vec::new();

					// Render custom templates
					if svc_ctx.depends_on_region_config() {
						build_region_config_template(&project_ctx, &mut templates).await;
					}

					json!(templates)
				}),
				// TODO: Add back
				// Give time for the blocking requests (60s long) to repeat, give another 15s for extra shutdown time
				// kill_timeout: Some(json!(chrono::secs(75))),
				kill_timeout: Some(json!(chrono::millis(100))),
				// TODO: Add back
				// Remove service from Consul before stopping the service in
				// order to ensure no requests get dropped
				// shutdown_delay: Some(chrono::secs(0)),
				shutdown_delay: Some(chrono::secs(0)),
				resources: Some(Resources {
					cpu: match ns_service_config.resources.cpu {
						config::ns::CpuResources::Cpu(x) => Some(x as i64),
						_ => None,
					},
					cores: match ns_service_config.resources.cpu {
						config::ns::CpuResources::CpuCores(x) => Some(x as i64),
						_ => None,
					},
					memory_mb: Some(ns_service_config.resources.memory as i64),
					..Default::default()
				}),
				log_config: Some(json!({
					"MaxFiles": 2,
					"MaxFileSizeMB": 4,
				})),

				volume_mounts: match driver {
					ExecServiceDriver::Docker { .. }
					| ExecServiceDriver::UploadedBinaryArtifact { .. } => None,
					// Mount the service binaries to execute directly in directly in the container. See
					// notes in salt/salt/nomad/files/nomad.d/client.hcl.j2.
					ExecServiceDriver::LocalBinaryArtifact { .. } => Some(json!([
						{
							"Volume": "backend-repo",
							"Destination": "/var/rivet/backend",
						},
						{
							"Volume": "nix-store",
							"Destination": "/nix/store",
						}
					])),
				},

				..Default::default()
			}]),

			volumes: match driver {
				ExecServiceDriver::Docker { .. }
				| ExecServiceDriver::UploadedBinaryArtifact { .. } => None,

				// Mount the service binaries to execute directly in directly in the container. See
				// notes in salt/salt/nomad/files/nomad.d/client.hcl.j2.
				ExecServiceDriver::LocalBinaryArtifact { .. } => Some(json!({
					"backend-repo": {
						"Type": "host",
						"Source": "backend-repo",
						"ReadOnly": true,
					},
					"nix-store": {
						"Type": "host",
						"Source": "nix-store",
						"ReadOnly": true,
					}
				})),
			},

			..Default::default()
		}]),
		..Default::default()
	}
}

mod chrono {
	pub fn min(x: i64) -> i64 {
		secs(x * 60)
	}

	pub fn secs(x: i64) -> i64 {
		millis(x * 1_000)
	}

	pub fn millis(x: i64) -> i64 {
		x * 1_000_000
	}
}

/// Define check restart config for connection health checks. On local, we want
/// the service to fail immediately so we can get immediate feedback if
/// something is wrong.
fn build_conn_check(name: &str, path: &str, interval: i64, on_update: &str) -> Check {
	// This check only affects Consul traffic. We don't restart the service here
	// because it would cause cascading failures if a database goes down.
	//
	// We need to check services frequently in order to quickly reroute traffic
	// to healthy nodes.

	Check {
		name: Some(name.into()),
		check_type: Some("http".into()),
		port_label: Some("health".into()),
		path: Some(path.into()),
		interval: Some(interval),
		timeout: Some(chrono::secs(5)),
		on_update: Some(on_update.into()),
		..Default::default()
	}
}

async fn build_region_config_template(
	ctx: &ProjectContext,
	templates: &mut Vec<serde_json::Value>,
) {
	let regions_json = serde_json::to_string(&ctx.ns().regions).unwrap();

	templates.push(json!({
		"DestPath": format!("local/region-config.json"),
		"EmbeddedTmpl": Some(regions_json),
	}));
}

/// Generates service tags that are provided to Treafik to determine how to route traffic to the
/// service.
///
/// See [here](https://doc.traefik.io/traefik/providers/nomad/) for more details.
fn build_router_service_tags(
	project_ctx: &ProjectContext,
	svc_ctx: &ServiceContext,
	router: &ServiceRouter,
	tags: &mut Vec<String>,
) {
	// Enable Traefik if there are mounts
	if !router.mounts.is_empty() {}

	// Register all mounts with Traefik
	for (i, mount) in router.mounts.iter().enumerate() {
		// Determine which router to mount on
		for prefix in ["traefik-ing-px", "traefik-local"] {
			// Enable if not already
			let enable_tag = format!("{prefix}.enable=true");
			if !tags.contains(&enable_tag) {
				tags.push(enable_tag);
			}

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
				middlewares.push(mw_name.clone());
				tags.extend([
					format!("{prefix}.http.middlewares.{mw_name}.stripPrefix.prefixes={path}"),
					format!("{prefix}.http.middlewares.{mw_name}.stripPrefix.forceSlash=true"),
				]);
			}

			// Compress
			{
				let mw_name = format!("{}-{i}-compress", svc_ctx.name());
				middlewares.push(mw_name.clone());
				tags.extend([format!("{prefix}.http.middlewares.{mw_name}.compress=true")]);
			}

			// In flight
			{
				let mw_name = format!("{}-{i}-inflight", svc_ctx.name());
				middlewares.push(mw_name.clone());
				tags.extend([format!(
					"{prefix}.http.middlewares.{mw_name}.inflightreq.amount=64"
				)]);
				tags.extend([format!("{prefix}.http.middlewares.{mw_name}.inflightreq.sourcecriterion.requestheadername=cf-connecting-ip")]);
			}

			// Build router
			let r_name = format!("{}-{i}", svc_ctx.name());
			tags.extend([
				format!("{prefix}.http.routers.{r_name}.entryPoints=lb-443"),
				format!("{prefix}.http.routers.{r_name}.rule={rule}"),
				format!(
					"{prefix}.http.routers.{r_name}.middlewares={}",
					middlewares.join(", ")
				),
				format!("{prefix}.http.routers.{r_name}.tls=true"),
			]);

			if svc_ctx.name() == "api-cf-verification" {
				tags.extend([
				format!("{prefix}.http.routers.cf-verification-challenge.rule=PathPrefix(`/.well-known/cf-custom-hostname-challenge/`)"),
				format!("{prefix}.http.routers.cf-verification-challenge.priority=90"),
			]);
			}
		}
	}
}

async fn nomad_docker_io_auth(ctx: &ProjectContext) -> Result<serde_json::Value> {
	if ctx.ns().docker.authenticate_all_docker_hub_pulls {
		let username = ctx
			.read_secret(&["docker", "docker_io", "username"])
			.await
			.unwrap();
		let password = ctx
			.read_secret(&["docker", "docker_io", "password"])
			.await
			.unwrap();
		Ok(json!({
			"username": username,
			"password": password,
		}))
	} else {
		Ok(json!(null))
	}
}
