use std::{
	collections::HashMap,
	convert::TryInto,
	hash::{DefaultHasher, Hasher},
	net::IpAddr,
};

use chirp_workflow::prelude::*;
use cluster::types::BuildDeliveryMethod;
use futures_util::FutureExt;
use nomad_client::models::*;
use rand::Rng;
use rivet_operation::prelude::proto::backend;
use serde_json::json;
use sha2::{Digest, Sha256};
use util::AsHashableExt;

use crate::{
	types::{
		build::{BuildCompression, BuildKind},
		GameGuardProtocol, NetworkMode, Routing, ServerResources,
	},
	util::{
		nomad_job::{
			escape_go_template, gen_oci_bundle_config, inject_consul_env_template,
			nomad_host_port_env_var, template_env_var_int, DecodedPort, TransportProtocol,
		},
		NOMAD_CONFIG, NOMAD_REGION,
	},
};

pub mod destroy;
pub mod nomad_alloc_plan;
pub mod nomad_alloc_update;
pub mod nomad_eval_update;

use nomad_eval_update::EvalStatus;

const SETUP_SCRIPT: &str = include_str!("./scripts/setup.sh");
const SETUP_JOB_RUNNER_SCRIPT: &str = include_str!("./scripts/setup_job_runner.sh");
const SETUP_OCI_BUNDLE_SCRIPT: &str = include_str!("./scripts/setup_oci_bundle.sh");
const SETUP_CNI_NETWORK_SCRIPT: &str = include_str!("./scripts/setup_cni_network.sh");

#[derive(Default, Clone)]
struct GameGuardUnnest {
	port_names: Vec<String>,
	port_numbers: Vec<Option<i32>>,
	gg_ports: Vec<Option<i32>>,
	protocols: Vec<i32>,
}

#[derive(Default, Clone)]
struct HostUnnest {
	port_names: Vec<String>,
	port_numbers: Vec<Option<i32>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Input {
	pub server_id: Uuid,
	pub env_id: Uuid,
	pub datacenter_id: Uuid,
	pub cluster_id: Uuid,
	pub tags: HashMap<String, String>,
	pub resources: ServerResources,
	pub kill_timeout_ms: i64,
	pub image_id: Uuid,
	pub args: Vec<String>,
	pub network_mode: NetworkMode,
	pub environment: HashMap<String, String>,
	pub network_ports: HashMap<String, Port>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
pub struct Port {
	// Null when using host networking since one is automatically assigned
	pub internal_port: Option<i32>,
	pub routing: Routing,
}

#[workflow]
pub async fn ds_server(ctx: &mut WorkflowCtx, input: &Input) -> GlobalResult<()> {
	let res = setup(ctx, input).await;
	match ctx.catch_unrecoverable(res)? {
		Ok(_) => {}
		// If we cannot recover a setup error, send a failed signal
		Err(err) => {
			tracing::warn!(?err, "unrecoverable setup");

			// TODO: Cleanup

			ctx.msg(CreateFailed {})
				.tag("server_id", input.server_id)
				.send()
				.await?;

			// Throw the original error from the setup activities
			return Err(err);
		}
	}

	ctx.msg(CreateComplete {})
		.tag("server_id", input.server_id)
		.send()
		.await?;

	// Wait for evaluation
	match ctx.listen::<Eval>().await? {
		Eval::NomadEvalUpdate(sig) => {
			let eval_status = ctx
				.workflow(nomad_eval_update::Input {
					server_id: input.server_id,
					eval: sig.eval,
				})
				.await?;

			if let EvalStatus::Failed = eval_status {
				tracing::info!("destroying after failed evaluation");

				ctx.workflow(destroy::Input {
					server_id: input.server_id,
					override_kill_timeout_ms: None,
				})
				.await?;
			}
		}
		Eval::Destroy(sig) => {
			tracing::info!("destroying before evaluation");

			ctx.workflow(destroy::Input {
				server_id: input.server_id,
				override_kill_timeout_ms: sig.override_kill_timeout_ms,
			})
			.await?;

			return Ok(());
		}
	}

	let override_kill_timeout_ms = ctx
		.repeat(|ctx| {
			let server_id = input.server_id;

			async move {
				match ctx.listen::<Main>().await? {
					Main::NomadAllocPlan(sig) => {
						ctx.workflow(nomad_alloc_plan::Input {
							server_id,
							alloc: sig.alloc,
						})
						.await?;
					}
					Main::NomadAllocUpdate(sig) => {
						let finished = ctx
							.workflow(nomad_alloc_update::Input {
								server_id,
								alloc: sig.alloc,
							})
							.await?;

						if finished {
							tracing::info!("alloc finished");
							return Ok(Loop::Break(None));
						}
					}
					Main::Destroy(sig) => return Ok(Loop::Break(sig.override_kill_timeout_ms)),
				}

				Ok(Loop::Continue)
			}
			.boxed()
		})
		.await?;

	ctx.workflow(destroy::Input {
		server_id: input.server_id,
		override_kill_timeout_ms,
	})
	.run()
	.await?;

	Ok(())
}

async fn setup(ctx: &mut WorkflowCtx, input: &Input) -> GlobalResult<()> {
	let (_, prereq) = ctx
		.join((
			InsertDbInput {
				server_id: input.server_id,
				env_id: input.env_id,
				datacenter_id: input.datacenter_id,
				cluster_id: input.cluster_id,
				tags: input.tags.as_hashable(),
				resources: input.resources.clone(),
				kill_timeout_ms: input.kill_timeout_ms,
				image_id: input.image_id,
				args: input.args.clone(),
				network_mode: input.network_mode,
				environment: input.environment.as_hashable(),
				network_ports: input.network_ports.as_hashable(),
			},
			GetBuildAndDcInput {
				image_id: input.image_id,
				datacenter_id: input.datacenter_id,
			},
		))
		.await?;

	let job_id = ctx
		.activity(SubmitJobInput {
			datacenter_id: input.datacenter_id,
			resources: input.resources.clone(),
			network_mode: input.network_mode,
			network_ports: input.network_ports.as_hashable(),
			build_kind: prereq.build_kind,
			build_compression: prereq.build_compression,
			dc_name_id: prereq.dc_name_id,
		})
		.await?;

	let artifacts = ctx
		.activity(ResolveArtifactsInput {
			datacenter_id: input.datacenter_id,
			image_id: input.image_id,
			server_id: input.server_id,
			build_upload_id: prereq.build_upload_id,
			build_file_name: prereq.build_file_name,
			dc_build_delivery_method: prereq.dc_build_delivery_method,
		})
		.await?;

	let nomad_dispatched_job_id = ctx
		.activity(DispatchJobInput {
			environment: input.environment.as_hashable(),
			server_id: input.server_id,
			build_upload_id: prereq.build_upload_id,
			job_id,
			image_artifact_url: artifacts.image_artifact_url,
			job_runner_binary_url: artifacts.job_runner_binary_url,
		})
		.await?;

	ctx.activity(UpdateDbInput {
		server_id: input.server_id,
		nomad_dispatched_job_id,
	})
	.await?;

	Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
struct InsertDbInput {
	server_id: Uuid,
	env_id: Uuid,
	datacenter_id: Uuid,
	cluster_id: Uuid,
	tags: util::HashableMap<String, String>,
	resources: ServerResources,
	kill_timeout_ms: i64,
	image_id: Uuid,
	args: Vec<String>,
	network_mode: NetworkMode,
	environment: util::HashableMap<String, String>,
	network_ports: util::HashableMap<String, Port>,
}

#[activity(InsertDb)]
async fn insert_db(ctx: &ActivityCtx, input: &InsertDbInput) -> GlobalResult<()> {
	let mut gg_unnest = GameGuardUnnest::default();
	let mut host_unnest = HostUnnest::default();

	for (name, port) in input.network_ports.iter() {
		match port.routing {
			Routing::GameGuard { protocol } => {
				gg_unnest.port_names.push(name.clone());
				gg_unnest.port_numbers.push(port.internal_port);
				gg_unnest.gg_ports.push(if port.internal_port.is_some() {
					Some(choose_ingress_port(ctx, protocol).await?)
				} else {
					None
				});
				gg_unnest.protocols.push(protocol as i32);
			}
			Routing::Host { .. } => {
				host_unnest.port_names.push(name.clone());
				host_unnest.port_numbers.push(port.internal_port);
			}
		};
	}

	// Run in a transaction for retryability
	rivet_pools::utils::crdb::tx(&ctx.crdb().await?, |tx| {
		let ctx = ctx.clone();
		let input = input.clone();
		let host_unnest = host_unnest.clone();
		let gg_unnest = gg_unnest.clone();

		async move {
			sql_execute!(
				[ctx, @tx tx]
				"
				WITH
					server AS (
						INSERT INTO db_ds.servers (
							server_id,
							env_id,
							datacenter_id,
							cluster_id,
							tags,
							resources_cpu_millicores,
							resources_memory_mib,
							kill_timeout_ms,
							create_ts,
							image_id,
							args,
							network_mode,
							environment
						)
						VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
						RETURNING 1
					),
					host_port AS (
						INSERT INTO db_ds.docker_ports_host (
							server_id,
							port_name,
							port_number
						)
						SELECT $1, t.*
						FROM unnest($14, $15) AS t(port_name, port_number)
						RETURNING 1
					),
					gg_port AS (
						INSERT INTO db_ds.docker_ports_protocol_game_guard (
							server_id,
							port_name,
							port_number,
							gg_port,
							protocol
						)
						SELECT $1, t.*
						FROM unnest($16, $17, $18, $19) AS t(port_name, port_number, gg_port, protocol)
						-- Check if lists are empty
						WHERE port_name IS NOT NULL
						RETURNING 1
					)
				SELECT 1
				",
				input.server_id,
				input.env_id,
				input.datacenter_id,
				input.cluster_id,
				serde_json::to_string(&input.tags)?, // 5
				input.resources.cpu_millicores,
				input.resources.memory_mib,
				input.kill_timeout_ms,
				ctx.ts(),
				input.image_id, // 10
				&input.args,
				input.network_mode as i32,
				serde_json::to_string(&input.environment)?,
				host_unnest.port_names,
				host_unnest.port_numbers, // 15
				gg_unnest.port_names,
				gg_unnest.port_numbers,
				gg_unnest.gg_ports,
				gg_unnest.protocols,
			)
			.await
		}
		.boxed()
	})
	.await?;

	// NOTE: This call is infallible because redis is infallible. If it was not, it would be put in its own
	// workflow step
	// Invalidate cache when new server is created
	ctx.cache()
		.purge("servers_ports", [input.datacenter_id])
		.await?;

	Ok(())
}

#[derive(Debug, Serialize, Deserialize, Hash)]
struct GetBuildAndDcInput {
	datacenter_id: Uuid,
	image_id: Uuid,
}

#[derive(Debug, Serialize, Deserialize, Hash)]
struct GetBuildAndDcOutput {
	build_upload_id: Uuid,
	build_file_name: String,
	build_kind: BuildKind,
	build_compression: BuildCompression,
	dc_name_id: String,
	dc_build_delivery_method: BuildDeliveryMethod,
}

#[activity(GetBuildAndDc)]
async fn get_build_and_dc(
	ctx: &ActivityCtx,
	input: &GetBuildAndDcInput,
) -> GlobalResult<GetBuildAndDcOutput> {
	// Validate build exists and belongs to this game
	let (build_res, dc_res) = tokio::try_join!(
		op!([ctx] build_get {
			build_ids: vec![input.image_id.into()],
		}),
		ctx.op(cluster::ops::datacenter::get::Input {
			datacenter_ids: vec![input.datacenter_id],
		})
	)?;
	let build = unwrap!(build_res.builds.first());
	let upload_id = unwrap!(build.upload_id).as_uuid();
	let build_kind = unwrap!(backend::build::BuildKind::from_i32(build.kind));
	let build_compression = unwrap!(backend::build::BuildCompression::from_i32(
		build.compression
	));

	let dc = unwrap!(dc_res.datacenters.first());

	Ok(GetBuildAndDcOutput {
		build_upload_id: upload_id,
		build_file_name: util_build::file_name(build_kind, build_compression),
		build_kind: unwrap!(BuildKind::from_repr(build.kind.try_into()?)),
		build_compression: unwrap!(BuildCompression::from_repr(build.compression.try_into()?)),
		dc_name_id: dc.name_id.clone(),
		dc_build_delivery_method: dc.build_delivery_method,
	})
}

#[derive(Debug, Serialize, Deserialize, Hash)]
struct SubmitJobInput {
	datacenter_id: Uuid,
	resources: ServerResources,
	network_mode: NetworkMode,
	network_ports: util::HashableMap<String, Port>,
	build_kind: BuildKind,
	build_compression: BuildCompression,
	dc_name_id: String,
}

#[activity(SubmitJob)]
async fn submit_job(ctx: &ActivityCtx, input: &SubmitJobInput) -> GlobalResult<String> {
	let tier_res = op!([ctx] tier_list {
		region_ids: vec![input.datacenter_id.into()],
	})
	.await?;
	let tier_region = unwrap!(tier_res.regions.first());

	// Find the first tier that has more CPU and memory than the requested
	// resources
	let mut tiers = tier_region.tiers.clone();

	// Sort the tiers by cpu
	tiers.sort_by(|a, b| a.cpu.cmp(&b.cpu));
	let tier = unwrap!(tiers.iter().find(|t| {
		t.cpu as i32 >= input.resources.cpu_millicores
			&& t.memory as i32 >= input.resources.memory_mib
	}));

	// runc-compatible resources
	let cpu = tier.rivet_cores_numerator as u64 * 1_000 / tier.rivet_cores_denominator as u64; // Millicore (1/1000 of a core)
	let memory = tier.memory * (1024 * 1024); // bytes
	let memory_max = tier.memory_max * (1024 * 1024); // bytes

	// Nomad-compatible resources
	let nomad_resources = Resources {
		// TODO: Configure this per-provider
		// Nomad configures CPU based on MHz, not millicores. We havel to calculate the CPU share
		// by knowing how many MHz are on the client.
		CPU: if tier.rivet_cores_numerator < tier.rivet_cores_denominator {
			Some((tier.cpu - util_job::TASK_CLEANUP_CPU as u64).try_into()?)
		} else {
			None
		},
		cores: if tier.rivet_cores_numerator >= tier.rivet_cores_denominator {
			Some((tier.rivet_cores_numerator / tier.rivet_cores_denominator) as i32)
		} else {
			None
		},
		memory_mb: Some(
			(TryInto::<i64>::try_into(memory)? / (1024 * 1024)
				- util_job::TASK_CLEANUP_MEMORY as i64)
				.try_into()?,
		),
		// Allow oversubscribing memory by 50% of the reserved
		// memory if using less than the node's total memory
		memory_max_mb: None,
		// Some(
		// 	(TryInto::<i64>::try_into(memory_max)? / (1024 * 1024)
		// 		- util_job::TASK_CLEANUP_MEMORY as i64)
		// 		.try_into()?,
		// ),
		disk_mb: Some(tier.disk as i32), // TODO: Is this deprecated?
		..Resources::new()
	};

	// Read ports
	let decoded_ports = input
		.network_ports
		.iter()
		.map(|(port_label, port)| match port.routing {
			Routing::GameGuard { protocol } => {
				let target = unwrap!(port.internal_port) as u16;

				Ok(DecodedPort {
					label: port_label.clone(),
					nomad_port_label: crate::util::format_nomad_port_label(port_label),
					target,
					proxy_protocol: protocol,
				})
			}
			Routing::Host { .. } => {
				todo!()
			}
		})
		.collect::<GlobalResult<Vec<DecodedPort>>>()?;

	// The container will set up port forwarding manually from the Nomad-defined ports on the host
	// to the CNI container
	let dynamic_ports = decoded_ports
		.iter()
		.map(|port| nomad_client::models::Port {
			label: Some(port.nomad_port_label.clone()),
			..nomad_client::models::Port::new()
		})
		.collect::<Vec<_>>();

	// Port mappings to pass to the container. Only used in bridge networking.
	let cni_port_mappings = decoded_ports
		.clone()
		.into_iter()
		.map(|port| {
			json!({
				"HostPort": template_env_var_int(&nomad_host_port_env_var(&port.nomad_port_label)),
				"ContainerPort": port.target,
				"Protocol": TransportProtocol::from(port.proxy_protocol).as_cni_protocol(),
			})
		})
		.collect::<Vec<_>>();

	// TODO:
	// let prepared_ports = input.network_ports.iter().map(|(label, port)| {
	// 	let port_value = match input.network_mode {
	// 		// CNI will handle mapping the host port to the container port
	// 		NetworkMode::Bridge => unwrap!(port.internal_port).to_string(),
	// 		// The container needs to listen on the correct port
	// 		NetworkMode::Host => template_env_var(&nomad_host_port_env_var(&label)),
	// 	};

	// 	GlobalResult::Ok(Some(String::new()))
	// 	// TODO
	// 	// Port with the kebab case port key. Included for backward compatibility & for
	// 	// less confusion.
	// 	// Ok((format!("PORT_{}", port.label.replace('-', "_")), port_value))
	// });

	// Also see util_ds:consts::DEFAULT_ENV_KEYS
	let mut env = Vec::<(String, String)>::new()
		.into_iter()
		// TODO
		// .chain(if lobby_config {
		// 	Some((
		// 		"RIVET_LOBBY_CONFIG".to_string(),
		// 		template_env_var("NOMAD_META_LOBBY_CONFIG"),
		// 	))
		// } else {
		// 	None
		// })
		// .chain(if lobby_tags {
		// 	Some((
		// 		"RIVET_LOBBY_TAGS".to_string(),
		// 		template_env_var("NOMAD_META_LOBBY_TAGS"),
		// 	))
		// } else {
		// 	None
		// })
		.chain([(
			"RIVET_API_ENDPOINT".to_string(),
			util::env::origin_api().to_string(),
		)])
		// Ports
		// TODO
		// .chain(prepared_ports)
		// // Port ranges
		// .chain(
		// 	decoded_ports
		// 		.iter()
		// 		.filter_map(|port| {
		// 			if let PortTarget::Range { min, max } = &port.target {
		// 				let snake_port_label = port.label.replace('-', "_");
		// 				Some([
		// 					(
		// 						format!("PORT_RANGE_MIN_{}", snake_port_label),
		// 						min.to_string(),
		// 					),
		// 					(
		// 						format!("PORT_RANGE_MAX_{}", snake_port_label),
		// 						max.to_string(),
		// 					),
		// 				])
		// 			} else {
		// 				None
		// 			}
		// 		})
		// 		.flatten(),
		// )
		.map(|(k, v)| format!("{k}={v}"))
		.collect::<Vec<String>>();
	env.sort();

	let services = decoded_ports
		.iter()
		.map(|port| {
			let service_name = format!("${{NOMAD_META_LOBBY_ID}}-{}", port.label);
			Ok(Some(Service {
				provider: Some("nomad".into()),
				name: Some(service_name),
				tags: Some(vec!["game".into()]),
				port_label: Some(port.nomad_port_label.clone()),
				// checks: if TransportProtocol::from(port.proxy_protocol)
				// 	== TransportProtocol::Tcp
				// {
				// 	Some(vec![ServiceCheck {
				// 		name: Some(format!("{}-probe", port.label)),
				// 		port_label: Some(port.nomad_port_label.clone()),
				// 		_type: Some("tcp".into()),
				// 		interval: Some(30_000_000_000),
				// 		timeout: Some(2_000_000_000),
				// 		..ServiceCheck::new()
				// 	}])
				// } else {
				// 	None
				// },
				..Service::new()
			}))
		})
		.filter_map(|x| x.transpose())
		.collect::<GlobalResult<Vec<_>>>()?;

	// Generate the command to download and decompress the file
	let mut download_cmd = r#"curl -Lf "$NOMAD_META_IMAGE_ARTIFACT_URL""#.to_string();
	match input.build_compression {
		BuildCompression::None => {}
		BuildCompression::Lz4 => {
			download_cmd.push_str(" | lz4 -d -");
		}
	}

	// IMPORTANT: This job spec must be deterministic. Do not pass in parameters
	// that change with every run, such as the lobby ID. Ensure the
	// `reuse_job_id` test passes when changing this function.
	let mut job_spec = Job {
		_type: Some("batch".into()),
		// Replace all job IDs with a placeholder value in order to create a
		// deterministic job spec for generating a hash
		ID: Some("__PLACEHOLDER__".into()),
		name: Some("__PLACEHOLDER__".into()),
		region: Some(NOMAD_REGION.into()),
		datacenters: Some(vec![input.datacenter_id.to_string()]),
		// constraints: Some(vec![Constraint {
		// 	l_target: Some("${node.class}".into()),
		// 	operand: Some("=".into()),
		// 	r_target: Some("job".into()),
		// }]),
		parameterized_job: Some(Box::new(ParameterizedJobConfig {
			payload: Some("forbidden".into()),
			meta_required: Some(vec![
				"job_runner_binary_url".into(),
				"vector_socket_addr".into(),
				"image_artifact_url".into(),
				"root_user_enabled".into(),
				"manager".into(),
				"user_env".into(),
				"server_id".into(),
			]),
			meta_optional: Some(vec!["rivet_test_id".into()]),
		})),
		task_groups: Some(vec![TaskGroup {
			name: Some(util_job::RUN_MAIN_TASK_NAME.into()),
			constraints: None, // TODO: Use parameter meta to specify the hardware
			affinities: None,  // TODO:
			// Allows for jobs to keep running and receiving players in the
			// event of a disconnection from the Nomad server.
			max_client_disconnect: Some(5 * 60 * 1_000_000_000),
			restart_policy: Some(Box::new(RestartPolicy {
				attempts: Some(0),
				mode: Some("fail".into()),
				..RestartPolicy::new()
			})),
			reschedule_policy: Some(Box::new(ReschedulePolicy {
				attempts: Some(0),
				unlimited: Some(false),
				..ReschedulePolicy::new()
			})),
			networks: Some(vec![NetworkResource {
				// The setup.sh script will set up a CNI network if using bridge networking
				mode: Some("host".into()),
				dynamic_ports: Some(dynamic_ports.clone()),
				// Disable IPv6 DNS since Docker doesn't support IPv6 yet
				DNS: Some(Box::new(nomad_client::models::DnsConfig {
					servers: Some(vec![
						// Google
						"8.8.8.8".into(),
						"8.8.4.4".into(),
						"2001:4860:4860::8888".into(),
						"2001:4860:4860::8844".into(),
					]),
					// Disable default search from the host
					searches: Some(Vec::new()),
					options: Some(vec!["rotate".into(), "edns0".into(), "attempts:2".into()]),
				})),
				..NetworkResource::new()
			}]),
			services: Some(services),
			// Configure ephemeral disk for logs
			ephemeral_disk: Some(Box::new(EphemeralDisk {
				size_mb: Some(tier.disk as i32),
				..EphemeralDisk::new()
			})),
			tasks: Some(vec![
				// TODO
				Task {
					name: Some("runc-setup".into()),
					lifecycle: Some(Box::new(TaskLifecycle {
						hook: Some("prestart".into()),
						sidecar: Some(false),
					})),
					driver: Some("raw_exec".into()),
					config: Some({
						let mut x = HashMap::new();
						x.insert("command".into(), json!("${NOMAD_TASK_DIR}/setup.sh"));
						x
					}),
					templates: Some(vec![
						Template {
							embedded_tmpl: Some(SETUP_SCRIPT.replace(
								"__HOST_NETWORK__",
								match input.network_mode {
									NetworkMode::Bridge => "false",
									NetworkMode::Host => "true",
								},
							)),
							dest_path: Some("${NOMAD_TASK_DIR}/setup.sh".into()),
							perms: Some("744".into()),
							..Template::new()
						},
						Template {
							embedded_tmpl: Some(SETUP_JOB_RUNNER_SCRIPT.into()),
							dest_path: Some("${NOMAD_TASK_DIR}/setup_job_runner.sh".into()),
							perms: Some("744".into()),
							..Template::new()
						},
						Template {
							embedded_tmpl: Some(
								SETUP_OCI_BUNDLE_SCRIPT
									.replace("__DOWNLOAD_CMD__", &download_cmd)
									.replace(
										"__BUILD_KIND__",
										match input.build_kind {
											BuildKind::DockerImage => "docker-image",
											BuildKind::OciBundle => "oci-bundle",
										},
									),
							),
							dest_path: Some("${NOMAD_TASK_DIR}/setup_oci_bundle.sh".into()),
							perms: Some("744".into()),
							..Template::new()
						},
						Template {
							embedded_tmpl: Some(SETUP_CNI_NETWORK_SCRIPT.into()),
							dest_path: Some("${NOMAD_TASK_DIR}/setup_cni_network.sh".into()),
							perms: Some("744".into()),
							..Template::new()
						},
						Template {
							embedded_tmpl: Some(gen_oci_bundle_config(
								cpu, memory, memory_max, env,
							)?),
							dest_path: Some(
								"${NOMAD_ALLOC_DIR}/oci-bundle-config.base.json".into(),
							),
							..Template::new()
						},
						Template {
							embedded_tmpl: Some(inject_consul_env_template(
								&serde_json::to_string(&cni_port_mappings)?,
							)?),
							dest_path: Some("${NOMAD_ALLOC_DIR}/cni-port-mappings.json".into()),
							..Template::new()
						},
					]),
					resources: Some(Box::new(Resources {
						CPU: Some(crate::util::RUNC_SETUP_CPU),
						memory_mb: Some(crate::util::RUNC_SETUP_MEMORY),
						..Resources::new()
					})),
					log_config: Some(Box::new(LogConfig {
						max_files: Some(4),
						max_file_size_mb: Some(2),
						disabled: None,
					})),
					..Task::new()
				},
				// TODO
				Task {
					name: Some(util_job::RUN_MAIN_TASK_NAME.into()),
					driver: Some("raw_exec".into()),
					config: Some({
						let mut x = HashMap::new();
						// This is downloaded in setup_job_runner.sh
						x.insert("command".into(), json!("${NOMAD_ALLOC_DIR}/job-runner"));
						x
					}),
					resources: Some(Box::new(nomad_resources.clone())),
					// Intentionally high timeout. Killing jobs is handled manually with signals.
					kill_timeout: Some(86400 * 1_000_000_000),
					kill_signal: Some("SIGTERM".into()),
					log_config: Some(Box::new(LogConfig {
						max_files: Some(4),
						max_file_size_mb: Some(4),
						disabled: None,
					})),
					..Task::new()
				},
				// TODO: Remove
				// Task {
				// 	name: Some("runc-cleanup".into()),
				// 	lifecycle: Some(Box::new(TaskLifecycle {
				// 		hook: Some("poststop".into()),
				// 		sidecar: Some(false),
				// 	})),
				// 	driver: Some("raw_exec".into()),
				// 	config: Some({
				// 		let mut x = HashMap::new();
				// 		x.insert("command".into(), json!("${NOMAD_TASK_DIR}/cleanup.sh"));
				// 		x
				// 	}),
				// 	templates: Some(vec![Template {
				// 		embedded_tmpl: Some(CLEANUP_SCRIPT.into()),
				// 		dest_path: Some("${NOMAD_TASK_DIR}/cleanup.sh".into()),
				// 		perms: Some("744".into()),
				// 		..Template::new()
				// 	}]),
				// 	resources: Some(Box::new(Resources {
				// 		CPU: Some(util_mm::RUNC_CLEANUP_CPU),
				// 		memory_mb: Some(util_mm::RUNC_CLEANUP_MEMORY),
				// 		..Resources::new()
				// 	})),
				// 	log_config: Some(Box::new(LogConfig {
				// 		max_files: Some(4),
				// 		max_file_size_mb: Some(2),
				// 	})),
				// 	..Task::new()
				// },
				// Run cleanup task
				Task {
					name: Some(util_job::RUN_CLEANUP_TASK_NAME.into()),
					lifecycle: Some(Box::new(TaskLifecycle {
						hook: Some("poststop".into()),
						sidecar: Some(false),
					})),
					driver: Some("docker".into()),
					config: Some({
						let mut config = HashMap::new();

						config.insert("image".into(), json!("python:3.10.7-alpine3.16"));
						config.insert(
							"args".into(),
							json!([
								"/bin/sh",
								"-c",
								"apk add --no-cache ca-certificates && python3 /local/cleanup.py"
							]),
						);

						config
					}),
					templates: Some(vec![Template {
						dest_path: Some("local/cleanup.py".into()),
						embedded_tmpl: Some(formatdoc!(
							r#"
							import ssl
							import urllib.request, json, os, mimetypes, sys
			
							BEARER = '{{{{env "NOMAD_META_JOB_RUN_TOKEN"}}}}'
			
							ctx = ssl.create_default_context()
			
							def eprint(*args, **kwargs):
								print(*args, file=sys.stderr, **kwargs)
			
							def req(method, url, data = None, headers = {{}}):
								request = urllib.request.Request(
									url=url,
									data=data,
									method=method,
									headers=headers
								)
			
								try:
									res = urllib.request.urlopen(request, context=ctx)
									assert res.status == 200, f"Received non-200 status: {{res.status}}"
									return res
								except urllib.error.HTTPError as err:
									eprint(f"HTTP Error ({{err.code}} {{err.reason}}):\n\nBODY:\n{{err.read().decode()}}\n\nHEADERS:\n{{err.headers}}")
			
									raise err
			
							print(f'\n> Cleaning up job')
			
							res_json = None
							with req('POST', f'{origin_api}/job/runs/cleanup',
								data = json.dumps({{}}).encode(),
								headers = {{
									'Authorization': f'Bearer {{BEARER}}',
									'Content-Type': 'application/json'
								}}
							) as res:
								res_json = json.load(res)
			
			
							print('\n> Finished')
							"#,
							origin_api = util::env::origin_api(),
						)),
						..Template::new()
					}]),
					resources: Some(Box::new(Resources {
						CPU: Some(util_job::TASK_CLEANUP_CPU),
						memory_mb: Some(util_job::TASK_CLEANUP_MEMORY),
						..Resources::new()
					})),
					log_config: Some(Box::new(LogConfig {
						max_files: Some(4),
						max_file_size_mb: Some(2),
						disabled: Some(false),
					})),
					..Task::new()
				},
			]),
			..TaskGroup::new()
		}]),
		..Job::new()
	};

	// Derive jobspec hash
	//
	// We serialize the JSON to a canonical string then take a SHA hash of the output.
	let job_cjson_str = match cjson::to_string(&job_spec) {
		Ok(x) => x,
		Err(err) => {
			tracing::error!(?err, "cjson serialization failed");
			bail!("cjson serialization failed")
		}
	};
	let job_hash = Sha256::digest(job_cjson_str.as_bytes());
	let job_hash_str = hex::encode(job_hash);

	// Generate new job ID
	let job_id = format!(
		"ds-{hash}:{dc}",
		hash = &job_hash_str[0..12],
		dc = input.dc_name_id,
	);
	job_spec.ID = Some(job_id.clone());
	job_spec.name = Some(job_id.clone());

	tracing::info!("submitting job");

	nomad_client::apis::jobs_api::post_job(
		&NOMAD_CONFIG,
		&job_id,
		nomad_client::models::JobRegisterRequest {
			job: Some(Box::new(job_spec)),
			..nomad_client::models::JobRegisterRequest::new()
		},
		Some(NOMAD_REGION),
		None,
		None,
		None,
	)
	.await?;

	Ok(job_id)
}

#[derive(Debug, Serialize, Deserialize, Hash)]
struct ResolveArtifactsInput {
	datacenter_id: Uuid,
	image_id: Uuid,
	server_id: Uuid,
	build_upload_id: Uuid,
	build_file_name: String,
	dc_build_delivery_method: BuildDeliveryMethod,
}

#[derive(Debug, Serialize, Deserialize, Hash)]
struct ResolveArtifactsOutput {
	image_artifact_url: String,
	job_runner_binary_url: String,
}

#[activity(ResolveArtifacts)]
async fn resolve_artifacts(
	ctx: &ActivityCtx,
	input: &ResolveArtifactsInput,
) -> GlobalResult<ResolveArtifactsOutput> {
	let upload_res = op!([ctx] upload_get {
		upload_ids: vec![input.build_upload_id.into()],
	})
	.await?;
	let upload = unwrap!(upload_res.uploads.first());
	let upload_id = unwrap_ref!(upload.upload_id).as_uuid();

	// Get provider
	let proto_provider = unwrap!(
		backend::upload::Provider::from_i32(upload.provider),
		"invalid upload provider"
	);
	let provider = match proto_provider {
		backend::upload::Provider::Minio => s3_util::Provider::Minio,
		backend::upload::Provider::Backblaze => s3_util::Provider::Backblaze,
		backend::upload::Provider::Aws => s3_util::Provider::Aws,
	};

	let image_artifact_url = resolve_image_artifact_url(
		ctx,
		input.datacenter_id,
		input.build_file_name.clone(),
		input.dc_build_delivery_method,
		input.image_id,
		upload_id,
		provider,
	)
	.await?;
	let job_runner_binary_url =
		resolve_job_runner_binary_url(ctx, input.datacenter_id, input.dc_build_delivery_method)
			.await?;

	// MARK: Insert into db
	sql_execute!(
		[ctx]
		"
		INSERT INTO db_ds.server_nomad (server_id)
		VALUES ($1)
		",
		input.server_id,
	)
	.await?;

	Ok(ResolveArtifactsOutput {
		image_artifact_url,
		job_runner_binary_url,
	})
}

#[derive(Debug, Serialize, Deserialize, Hash)]
struct DispatchJobInput {
	environment: util::HashableMap<String, String>,
	server_id: Uuid,
	job_id: String,
	build_upload_id: Uuid,
	image_artifact_url: String,
	job_runner_binary_url: String,
}

#[activity(DispatchJob)]
async fn dispatch_job(ctx: &ActivityCtx, input: &DispatchJobInput) -> GlobalResult<String> {
	let parameters = vec![
		backend::job::Parameter {
			key: "job_runner_binary_url".into(),
			value: input.job_runner_binary_url.clone(),
		},
		backend::job::Parameter {
			key: "vector_socket_addr".into(),
			value: "127.0.0.1:5021".to_string(),
		},
		backend::job::Parameter {
			key: "image_artifact_url".into(),
			value: input.image_artifact_url.clone(),
		},
		backend::job::Parameter {
			key: "root_user_enabled".into(),
			// TODO make table dynamic host, make reference so that we can find
			// other locations
			value: "0".into(),
		},
		backend::job::Parameter {
			key: "manager".into(),
			value: "dynamic_servers".into(),
		},
		backend::job::Parameter {
			key: "user_env".into(),
			// other locations
			value: unwrap!(serde_json::to_string(
				&input
					.environment
					.iter()
					.map(|(k, v)| (k.clone(), escape_go_template(v)))
					.collect::<HashMap<_, _>>(),
			)),
		},
	]
	.into_iter()
	.collect::<Vec<_>>();

	let job_params = vec![("server_id".to_string(), input.server_id.to_string())];

	// MARK: Dispatch job
	let dispatch_res = nomad_client::apis::jobs_api::post_job_dispatch(
		&NOMAD_CONFIG,
		&input.job_id,
		nomad_client::models::JobDispatchRequest {
			job_id: Some(input.job_id.clone()),
			payload: None,
			meta: Some(
				parameters
					.iter()
					.map(|p| (p.key.clone(), p.value.clone()))
					.chain(job_params.into_iter())
					.collect::<HashMap<String, String>>(),
			),
		},
		Some(NOMAD_REGION),
		None,
		None,
		None,
	)
	.await?;

	// We will use the dispatched job ID to identify this allocation for the future. We can't use
	// eval ID, since that changes if we mutate the allocation (i.e. try to stop it).
	let nomad_dispatched_job_id = unwrap_ref!(dispatch_res.dispatched_job_id);

	Ok(nomad_dispatched_job_id.clone())
}

#[derive(Debug, Serialize, Deserialize, Hash)]
struct UpdateDbInput {
	server_id: Uuid,
	nomad_dispatched_job_id: String,
}

#[activity(UpdateDb)]
async fn update_db(ctx: &ActivityCtx, input: &UpdateDbInput) -> GlobalResult<()> {
	// MARK: Write to db after run
	sql_execute!(
		[ctx]
		"
		UPDATE db_ds.server_nomad
		SET nomad_dispatched_job_id = $2
		WHERE server_id = $1
		",
		input.server_id,
		&input.nomad_dispatched_job_id,
	)
	.await?;

	ctx.update_workflow_tags(&json!({
		"server_id": input.server_id,
		"nomad_dispatched_job_id": input.nomad_dispatched_job_id,
	}))
	.await?;

	Ok(())
}

#[message("ds_server_create_complete")]
pub struct CreateComplete {}

#[message("ds_server_create_failed")]
pub struct CreateFailed {}

#[signal("ds_server_destroy")]
pub struct Destroy {
	pub override_kill_timeout_ms: Option<i64>,
}

#[signal("ds_server_nomad_alloc_plan")]
pub struct NomadAllocPlan {
	pub alloc: nomad_client::models::Allocation,
}

#[signal("ds_server_nomad_alloc_update")]
pub struct NomadAllocUpdate {
	pub alloc: nomad_client::models::Allocation,
}

#[signal("ds_server_nomad_eval_update")]
pub struct NomadEvalUpdate {
	pub eval: nomad_client::models::Evaluation,
}

join_signal!(Eval, [NomadEvalUpdate, Destroy,]);

join_signal!(Main, [NomadAllocPlan, NomadAllocUpdate, Destroy,]);

/// Choose which port to assign for a job's ingress port.
///
/// If not provided by `ProxiedPort`, then:
/// - HTTP: 80
/// - HTTPS: 443
/// - TCP/TLS: random
/// - UDP: random
///
/// This is somewhat poorly written for TCP & UDP ports and may bite us in the ass
/// some day. See https://linear.app/rivet-gg/issue/RIV-1799
async fn choose_ingress_port(ctx: &ActivityCtx, protocol: GameGuardProtocol) -> GlobalResult<i32> {
	match protocol {
		GameGuardProtocol::Http => Ok(80),
		GameGuardProtocol::Https => Ok(443),
		GameGuardProtocol::Tcp | GameGuardProtocol::TcpTls => {
			bind_with_retries(
				ctx,
				protocol,
				util::net::job::MIN_INGRESS_PORT_TCP..=util::net::job::MAX_INGRESS_PORT_TCP,
			)
			.await
		}
		GameGuardProtocol::Udp => {
			bind_with_retries(
				ctx,
				protocol,
				util::net::job::MIN_INGRESS_PORT_UDP..=util::net::job::MAX_INGRESS_PORT_UDP,
			)
			.await
		}
	}
}

async fn bind_with_retries(
	ctx: &ActivityCtx,
	proxy_protocol: GameGuardProtocol,
	range: std::ops::RangeInclusive<u16>,
) -> GlobalResult<i32> {
	let mut attempts = 3u32;

	// Try to bind to a random port, verifying that it is not already bound
	loop {
		if attempts == 0 {
			bail!("failed all attempts to bind to unique port");
		}
		attempts -= 1;

		let port = rand::thread_rng().gen_range(range.clone()) as i32;

		let (already_exists,) = sql_fetch_one!(
			[ctx, (bool,)]
			"
			SELECT EXISTS(
				SELECT 1
				FROM db_ds.servers AS r
				JOIN db_ds.docker_ports_protocol_game_guard AS p
				ON r.server_id = p.server_id
				WHERE
					r.cleanup_ts IS NULL AND
					p.gg_port = $1 AND
					p.protocol = $2
			)
			",
			port,
			proxy_protocol as i32,
		)
		.await?;

		if !already_exists {
			break Ok(port);
		}

		tracing::info!(?port, ?attempts, "port collision, retrying");
	}
}

/// Generates a presigned URL for the build image.
async fn resolve_image_artifact_url(
	ctx: &ActivityCtx,
	datacenter_id: Uuid,
	build_file_name: String,
	build_delivery_method: BuildDeliveryMethod,
	build_id: Uuid,
	upload_id: Uuid,
	provider: s3_util::Provider,
) -> GlobalResult<String> {
	// Build URL
	match build_delivery_method {
		BuildDeliveryMethod::S3Direct => {
			tracing::info!("using s3 direct delivery");

			// Build client
			let s3_client = s3_util::Client::from_env_opt(
				"bucket-build",
				provider,
				s3_util::EndpointKind::External,
			)
			.await?;

			let presigned_req = s3_client
				.get_object()
				.bucket(s3_client.bucket())
				.key(format!("{upload_id}/{build_file_name}"))
				.presigned(
					s3_util::aws_sdk_s3::presigning::config::PresigningConfig::builder()
						.expires_in(std::time::Duration::from_secs(15 * 60))
						.build()?,
				)
				.await?;

			let addr = presigned_req.uri().clone();

			let addr_str = addr.to_string();
			tracing::info!(addr = %addr_str, "resolved artifact s3 presigned request");

			Ok(addr_str)
		}
		BuildDeliveryMethod::TrafficServer => {
			tracing::info!("using traffic server delivery");

			// Hash build so that the ATS server that we download the build from is always the same one. This
			// improves cache hit rates and reduces download times.
			let mut hasher = DefaultHasher::new();
			hasher.write(build_id.as_bytes());
			let hash = hasher.finish() as i64;

			// NOTE: The algorithm for choosing the vlan_ip from the hash should match the one in
			// prewarm_ats.rs @ prewarm_ats_cache
			// Get vlan ip from build id hash for consistent routing
			let (ats_vlan_ip,) = sql_fetch_one!(
				[ctx, (IpAddr,)]
				"
				WITH sel AS (
					-- Select candidate vlan ips
					SELECT
						vlan_ip
					FROM db_cluster.servers
					WHERE
						datacenter_id = $1 AND
						pool_type2 = $2 AND
						vlan_ip IS NOT NULL AND
						install_complete_ts IS NOT NULL AND
						drain_ts IS NULL AND
						cloud_destroy_ts IS NULL
				)
				SELECT vlan_ip
				FROM sel
				-- Use mod to make sure the hash stays within bounds
				OFFSET abs($3 % GREATEST((SELECT COUNT(*) FROM sel), 1))
				LIMIT 1
				",
				&datacenter_id,
				serde_json::to_string(&cluster::types::PoolType::Ats)?,
				hash,
			)
			.await?;

			let addr = format!(
				"http://{vlan_ip}:8080/s3-cache/{provider}/{namespace}-bucket-build/{upload_id}/{build_file_name}",
				vlan_ip = ats_vlan_ip,
				provider = heck::KebabCase::to_kebab_case(provider.as_str()),
				namespace = util::env::namespace(),
				upload_id = upload_id,
			);

			tracing::info!(%addr, "resolved artifact s3 url");

			Ok(addr)
		}
	}
}

/// Generates a presigned URL for the job runner binary.
async fn resolve_job_runner_binary_url(
	ctx: &ActivityCtx,
	datacenter_id: Uuid,
	build_delivery_method: BuildDeliveryMethod,
) -> GlobalResult<String> {
	// Get provider
	let provider = s3_util::Provider::default()?;

	let file_name = std::env::var("JOB_RUNNER_BINARY_KEY")?;

	// Build URL
	match build_delivery_method {
		BuildDeliveryMethod::S3Direct => {
			tracing::info!("job runner using s3 direct delivery");

			// Build client
			let s3_client = s3_util::Client::from_env_opt(
				"bucket-infra-artifacts",
				provider,
				s3_util::EndpointKind::External,
			)
			.await?;
			let presigned_req = s3_client
				.get_object()
				.bucket(s3_client.bucket())
				.key(file_name)
				.presigned(
					s3_util::aws_sdk_s3::presigning::config::PresigningConfig::builder()
						.expires_in(std::time::Duration::from_secs(15 * 60))
						.build()?,
				)
				.await?;

			let addr = presigned_req.uri().clone();

			let addr_str = addr.to_string();
			tracing::info!(addr = %addr_str, "resolved job runner presigned request");

			Ok(addr_str)
		}
		BuildDeliveryMethod::TrafficServer => {
			tracing::info!("job runner using traffic server delivery");

			// Choose a random ATS node to pull from
			let (ats_vlan_ip,) = sql_fetch_one!(
				[ctx, (IpAddr,)]
				"
				WITH sel AS (
					-- Select candidate vlan ips
					SELECT
						vlan_ip
					FROM db_cluster.servers
					WHERE
						datacenter_id = $1 AND
						pool_type2 = $2 AND
						vlan_ip IS NOT NULL AND
						install_complete_ts IS NOT NULL AND
						drain_ts IS NULL AND
						cloud_destroy_ts IS NULL	
				)
				SELECT vlan_ip
				FROM sel
				ORDER BY random()
				LIMIT 1
				",
				&datacenter_id,
				serde_json::to_string(&cluster::types::PoolType::Ats)?,
			)
			.await?;

			let addr = format!(
				"http://{vlan_ip}:8080/s3-cache/{provider}/{namespace}-bucket-infra-artifacts/{file_name}",
				vlan_ip = ats_vlan_ip,
				provider = heck::KebabCase::to_kebab_case(provider.as_str()),
				namespace = util::env::namespace(),
			);

			tracing::info!(%addr, "resolved artifact s3 url");

			Ok(addr)
		}
	}
}
