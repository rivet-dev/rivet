use std::{
	collections::HashMap,
	hash::{DefaultHasher, Hasher},
	net::IpAddr,
	time::Duration,
};

use crate::sqlx;
use futures_util::FutureExt;
use nomad_client::models::*;
use nomad_job::{
	escape_go_template, gen_oci_bundle_config, inject_consul_env_template, nomad_host_port_env_var,
	template_env_var, template_env_var_int, DecodedPort, ProxyProtocol, TransportProtocol,
};
use proto::{
	backend::{self, pkg::*},
	chirp::response::Ok,
};
use rand::Rng;
use regex::Regex;
use rivet_operation::prelude::*;
use serde_json::json;
use sha2::{Digest, Sha256};
use team::member_get::request;

mod nomad_job;
mod oci_config;
mod seccomp;
mod util_job;

lazy_static::lazy_static! {
	pub static ref NEW_NOMAD_CONFIG: nomad_client::apis::configuration::Configuration =
		nomad_util::new_config_from_env().unwrap();
}

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
async fn choose_ingress_port(
	ctx: OperationContext<dynamic_servers::server_create::Request>,
	ingress_port: i32,
	protocol: i32,
) -> GlobalResult<i32> {
	use backend::job::ProxyProtocol;

	let ingress_port = match unwrap!(backend::job::ProxyProtocol::from_i32(protocol)) {
		ProxyProtocol::Http => 80_i32,
		ProxyProtocol::Https => 443,
		ProxyProtocol::Tcp | ProxyProtocol::TcpTls => {
			bind_with_retries(
				ctx,
				protocol,
				util::net::job::MIN_INGRESS_PORT_TCP..=util::net::job::MAX_INGRESS_PORT_TCP,
			)
			.await?
		}
		ProxyProtocol::Udp => {
			bind_with_retries(
				ctx,
				protocol,
				util::net::job::MIN_INGRESS_PORT_UDP..=util::net::job::MAX_INGRESS_PORT_UDP,
			)
			.await?
		}
	};

	Ok(ingress_port)
}

async fn bind_with_retries(
	ctx: OperationContext<dynamic_servers::server_create::Request>,
	proxy_protocol: i32,
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
				FROM db_dynamic_servers.servers as r
				JOIN db_dynamic_servers.docker_ports_protocol_game_guard as p
				ON r.server_id = p.server_id
				WHERE
					r.cleanup_ts IS NULL AND
					p.gg_port = $1 AND
					p.protocol = $2
			)
			",
			port,
			proxy_protocol,
		)
		.await?;

		if !already_exists {
			break Ok(port);
		}

		tracing::info!(?port, ?attempts, "port collision, retrying");
	}
}

#[operation(name = "ds-server-create")]
pub async fn handle(
	ctx: OperationContext<dynamic_servers::server_create::Request>,
) -> GlobalResult<dynamic_servers::server_create::Response> {
	let resources = unwrap_ref!(ctx.resources).clone();
	let server_id = Uuid::new_v4();
	let game_id = unwrap_ref!(ctx.game_id).as_uuid();
	let cluster_id = unwrap_ref!(ctx.cluster_id).as_uuid();
	let datacenter_id = unwrap_ref!(ctx.datacenter_id).as_uuid();

	let create_ts = ctx.ts();

	// MARK: db insert

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

	let mut game_guard_unnest = GameGuardUnnest::default();
	let mut host_unnest = HostUnnest::default();

	for (name, port) in ctx.network_ports.iter() {
		let routing = unwrap!(port.routing.clone());
		match routing {
			dynamic_servers::server_create::port::Routing::GameGuard(gameguard_protocol) => {
				game_guard_unnest.port_names.push(name.clone());
				game_guard_unnest.port_numbers.push(port.internal_port);
				game_guard_unnest.gg_ports.push(match port.internal_port {
					Some(port) => Some(
						choose_ingress_port(ctx.clone(), port, gameguard_protocol.protocol).await?,
					),
					None => None,
				});
				game_guard_unnest
					.protocols
					.push(gameguard_protocol.protocol);
			}
			dynamic_servers::server_create::port::Routing::Host(_) => {
				host_unnest.port_names.push(name.clone());
				host_unnest.port_numbers.push(port.internal_port);
			}
		};
	}

	rivet_pools::utils::crdb::tx(&ctx.crdb().await?, |tx| {
		let ctx = ctx.clone();
		let host_unnest = host_unnest.clone();
		let game_guard_unnest = game_guard_unnest.clone();

		async move {
			sql_execute!(
				[ctx, @tx tx]
				"
				WITH
					servers_cte AS (
						INSERT INTO
							db_dynamic_servers.servers (
								server_id,
								game_id,
								datacenter_id,
								cluster_id,
								tags,
								resources_cpu_millicores,
								resources_memory_mib,
								kill_timeout_ms,
								webhook_url,
								create_ts,
								image_id,
								args,
								network_mode,
								environment
							)
						VALUES
							($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
						RETURNING
							1
					),
					docker_ports_host_cte AS (
						INSERT INTO
							db_dynamic_servers.docker_ports_host (
								server_id,
								port_name,
								port_number
							)
						SELECT
							$1,
							t.*
						FROM
							unnest($15, $16) AS t (port_name, port_number)
						RETURNING
							1
					),
					docker_ports_protocol_game_guard_cte AS (
						INSERT INTO
							db_dynamic_servers.docker_ports_protocol_game_guard (
								server_id,
								port_name,
								port_number,
								gg_port,
								protocol
							)
						SELECT
							$1,
							t.*
						FROM
							unnest($17, $18, $19, $20) AS t (port_name, port_number, protocol)
						RETURNING
							1
					)
				SELECT
					1
				",
				server_id,
				game_id,
				datacenter_id,
				cluster_id,
				serde_json::value::to_raw_value(&ctx.tags.to_owned())?.to_string(), // 5
				resources.cpu_millicores,
				resources.memory_mib,
				ctx.kill_timeout_ms,
				ctx.webhook_url.clone(),
				create_ts, // 10
				unwrap!(ctx.image_id).as_uuid(),
				&ctx.args,
				ctx.network_mode,
				serde_json::value::to_raw_value(&ctx.environment)?.to_string(),
				host_unnest.port_names, // 15
				host_unnest.port_numbers,
				game_guard_unnest.port_names,
				game_guard_unnest.port_numbers,
				game_guard_unnest.gg_ports,
				game_guard_unnest.protocols, // 20
			)
			.await
		}
		.boxed()
	})
	.await?;

	// let (
	// 	(mm_game_config, namespace),
	// 	mm_ns_config,
	// 	(lobby_group, lobby_group_meta, version_id),
	// 	region,
	// 	tiers,
	// ) = tokio::try_join!(
	// 	fetch_namespace(ctx, namespace_id),
	// 	fetch_mm_namespace_config(ctx, namespace_id),
	// 	fetch_lobby_group_config(ctx, lobby_group_id),
	// 	fetch_region(ctx, region_id),
	// 	fetch_tiers(ctx, region_id),
	// )?;
	// let (mm_game_config, namespace) = fetch_namespace(ctx, namespace_id).await?;
	// let mm_ns_config = fetch_mm_namespace_config(ctx, namespace_id).await?;
	// let (lobby_group, lobby_group_meta, version_id) = fetch_lobby_group_config(ctx, lobby_group_id)
	// 	.await?;
	// let region = fetch_region(ctx, region_id).await?;
	// let tiers = fetch_tiers(ctx, region_id).await?;
	// let version = fetch_version(ctx, version_id).await?;

	// // Do all nomad stuff
	// let namespace_id = unwrap_ref!(namespace.namespace_id).as_uuid();
	// let version_id = unwrap_ref!(version.version_id).as_uuid();
	// let lobby_group_id = unwrap_ref!(lobby_group_meta.lobby_group_id).as_uuid();
	// let region_id = unwrap_ref!(region.region_id).as_uuid();

	// let job_runner_binary_url = resolve_job_runner_binary_url(ctx).await?;

	// let resolve_perf = ctx.perf().start("resolve-image-artifact-url").await;
	// let build_id = unwrap_ref!(runtime.build_id).as_uuid();
	// let image_artifact_url = resolve_image_artifact_url(ctx, build_id, region).await?;
	// resolve_perf.end();

	// // Validate build exists and belongs to this game
	// let build_id = unwrap_ref!(runtime.build_id).as_uuid();
	// let build_get = op!([ctx] build_get {
	// 	build_ids: vec![build_id.into()],
	// })
	// .await?;
	// let build = unwrap!(build_get.builds.first());
	// let build_kind = unwrap!(backend::build::BuildKind::from_i32(build.kind));
	// let build_compression = unwrap!(backend::build::BuildCompression::from_i32(
	// 	build.compression
	// ));

	let ctx: OperationContext<dynamic_servers::server_create::Request> = ctx;

	// Generate the Docker job

	// let runtime = backend::ds::lobby_runtime::Docker {
	// 	build_id: todo!(),
	// 	args: docker_runtime.args,
	// 	env_vars: todo!(),
	// 	network_mode: todo!(),
	// 	ports: todo!(),
	// };
	// let _image_tag = &build.image_tag;
	// let tier = backend::region::Tier {
	// 	tier_name_id: todo!(),
	// 	rivet_cores_numerator: todo!(),
	// 	rivet_cores_denominator: todo!(),
	// 	cpu: todo!(),
	// 	memory: todo!(),
	// 	memory_max: todo!(),
	// 	disk: todo!(),
	// 	bandwidth: todo!(),
	// };

	// let lobby_config = ctx.lobby_config_json.is_some();
	// let lobby_tags = !ctx.tags.is_empty();
	// let build_kind = backend::build::BuildKind::DockerImage;
	// let build_compression = backend::build::BuildCompression::None;

	// IMPORTANT: This job spec must be deterministic. Do not pass in parameters
	// that change with every run, such as the lobby ID. Ensure the
	// `reuse_job_id` test passes when changing this function.
	use nomad_client::models::*;

	let resources = unwrap!(ctx.resources.clone());

	let tier_res = op!([ctx] tier_list {
		region_ids: vec![datacenter_id.into()],
	})
	.await?;
	let tier_region = unwrap!(tier_res.regions.first());

	// // runc-compatible resourcesd
	// let cpu = resources.cpu_millicores; // Millicore (1/1000 of a core)
	// let memory = resources.memory_mib * (1024 * 1024); // bytes
	// 											   // let memory_max = tier.memory_max * (1024 * 1024); // bytes

	// Find the first tier that has more CPU and memory than the requested
	// resources
	let mut tiers = tier_region.tiers.clone();

	// Sort the tiers by cpu
	tiers.sort_by(|a, b| a.cpu.cmp(&b.cpu));
	let tier = unwrap!(tiers.iter().find(|t| {
		t.cpu as i32 >= resources.cpu_millicores && t.memory as i32 >= resources.memory_mib
	}));

	// runc-compatible resources
	let cpu = tier.rivet_cores_numerator as u64 * 1_000 / tier.rivet_cores_denominator as u64; // Millicore (1/1000 of a core)
	let memory = tier.memory * (1024 * 1024); // bytes
	let memory_max = tier.memory_max * (1024 * 1024); // bytes

	// dbg!(tier, cpu, memory, memory_max);
	// panic!();

	// Validate build exists and belongs to this game
	let build_id = unwrap_ref!(ctx.image_id).as_uuid();
	let build_get = op!([ctx] build_get {
		build_ids: vec![build_id.into()],
	})
	.await?;
	let build = unwrap!(build_get.builds.first());
	let build_kind = unwrap!(backend::build::BuildKind::from_i32(build.kind));
	let build_compression = unwrap!(backend::build::BuildCompression::from_i32(
		build.compression
	));

	// // Nomad-compatible resources
	// let resources = Resources {
	// 	// TODO: Configure this per-provider
	// 	// Nomad configures CPU based on MHz, not millicores. We havel to calculate the CPU share
	// 	// by knowing how many MHz are on the client.
	// 	CPU: if cpu < 1000 {
	// 		Some((cpu - util_job::TASK_CLEANUP_CPU).try_into()?)
	// 	} else {
	// 		None
	// 	},
	// 	cores: if cpu >= 1000 {
	// 		Some((cpu / 1000) as i32)
	// 	} else {
	// 		None
	// 	},
	// 	memory_mb: Some(
	// 		(TryInto::<i64>::try_into(memory)? / (1024 * 1024)
	// 			- util_job::TASK_CLEANUP_MEMORY as i64)
	// 			.try_into()?,
	// 	),
	// 	// Allow oversubscribing memory by 50% of the reserved
	// 	// memory if using less than the node's total memory
	// 	memory_max_mb: Some(
	// 		(TryInto::<i64>::try_into((memory as f64 * 1.5) as i64)? / (1024 * 1024)
	// 			- util_job::TASK_CLEANUP_MEMORY as i64)
	// 			.try_into()?,
	// 	),
	// 	..Resources::new()
	// };

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

	// // let network_mode = unwrap!(LobbyRuntimeNetworkMode::from_i32(runtime.network_mode));

	// Read ports
	let decoded_ports = ctx
		.network_ports
		.clone()
		.into_iter()
		.map(|(port_label, port)| match port.routing {
			Some(dynamic_servers::server_create::port::Routing::GameGuard(game_guard_routing)) => {
				let target = unwrap!(port.internal_port) as u16;

				GlobalResult::Ok(DecodedPort {
					label: port_label.clone(),
					nomad_port_label: util_ds::format_nomad_port_label(&port_label),
					target,
					proxy_protocol: unwrap!(backend::ds::GameGuardProtocol::from_i32(
						game_guard_routing.protocol
					))
					.into(),
				})
			}
			Some(dynamic_servers::server_create::port::Routing::Host(_)) => {
				todo!()
			}
			None => {
				todo!()
			}
		})
		.collect::<GlobalResult<Vec<DecodedPort>>>()?;

	// The container will set up port forwarding manually from the Nomad-defined ports on the host
	// to the CNI container
	let dynamic_ports = decoded_ports
		.iter()
		.map(|port| Port {
			label: Some(port.nomad_port_label.clone()),
			..Port::new()
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

	let prepared_ports = ctx.network_ports.iter().map(|(label, port)| {
		let mode = unwrap!(backend::ds::NetworkMode::from_i32(ctx.network_mode));
		let port_value = match mode {
			// CNI will handle mapping the host port to the container port
			backend::ds::NetworkMode::Bridge => unwrap!(port.internal_port).to_string(),
			// The container needs to listen on the correct port
			backend::ds::NetworkMode::Host => template_env_var(&nomad_host_port_env_var(&label)),
		};

		GlobalResult::Ok(Some(String::new()))
		// TODO
		// Port with the kebab case port key. Included for backward compatabiilty & for
		// less confusion.
		// Ok((format!("PORT_{}", port.label.replace('-', "_")), port_value))
	});

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
			GlobalResult::Ok(Some(Service {
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
	match build_compression {
		backend::build::BuildCompression::None => {}
		backend::build::BuildCompression::Lz4 => {
			download_cmd.push_str(" | lz4 -d -");
		}
	}

	// MARK: Job spec

	let job_spec = Job {
		_type: Some("batch".into()),
		// constraints: Some(vec![Constraint {
		// 	l_target: Some("${node.class}".into()),
		// 	r_target: Some("job".into()),
		// 	operand: Some("=".into()),
		// }]),
		parameterized_job: Some(Box::new(ParameterizedJobConfig {
			payload: Some("forbidden".into()),
			meta_required: Some(vec![
				"job_runner_binary_url".into(),
				"vector_socket_addr".into(),
				"image_artifact_url".into(),
				"root_user_enabled".into(),
				"runner".into(),
				"user_env".into(),
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
							embedded_tmpl: Some(include_str!("./scripts/setup.sh").replace(
								"__HOST_NETWORK__",
								match unwrap!(backend::ds::NetworkMode::from_i32(ctx.network_mode))
								{
									backend::ds::NetworkMode::Bridge => "false",
									backend::ds::NetworkMode::Host => "true",
								},
							)),
							dest_path: Some("${NOMAD_TASK_DIR}/setup.sh".into()),
							perms: Some("744".into()),
							..Template::new()
						},
						Template {
							embedded_tmpl: Some(
								include_str!("./scripts/setup_job_runner.sh").into(),
							),
							dest_path: Some("${NOMAD_TASK_DIR}/setup_job_runner.sh".into()),
							perms: Some("744".into()),
							..Template::new()
						},
						Template {
							embedded_tmpl: Some(
								include_str!("./scripts/setup_oci_bundle.sh")
									.replace("__DOWNLOAD_CMD__", &download_cmd)
									.replace(
										"__BUILD_KIND__",
										match build_kind {
											backend::build::BuildKind::DockerImage => {
												"docker-image"
											}
											backend::build::BuildKind::OciBundle => "oci-bundle",
										},
									),
							),
							dest_path: Some("${NOMAD_TASK_DIR}/setup_oci_bundle.sh".into()),
							perms: Some("744".into()),
							..Template::new()
						},
						Template {
							embedded_tmpl: Some(
								include_str!("./scripts/setup_cni_network.sh").into(),
							),
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
						CPU: Some(util_ds::RUNC_SETUP_CPU),
						memory_mb: Some(util_ds::RUNC_SETUP_MEMORY),
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
				// 		embedded_tmpl: Some(include_str!("./scripts/cleanup.sh").into()),
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
			]),
			..TaskGroup::new()
		}]),
		// Disables rescheduling in the event of a node drain
		reschedule: Some(Box::new(ReschedulePolicy {
			attempts: Some(0),
			..ReschedulePolicy::new()
		})),
		..Job::new()
	};

	let job_spec_json = serde_json::to_string(&job_spec)?;

	// // Build proxied ports for each exposed port
	// let proxied_ports = runtime
	// 	.ports
	// 	.iter()
	// 	.filter(|port| {
	// 		port.proxy_kind == backend::ds::lobby_runtime::ProxyKind::GameGuard as i32
	// 			&& port.port_range.is_none()
	// 	})
	// 	.flat_map(|port| {
	// 		let mut ports = vec![direct_proxied_port(lobby_id, region_id, port)];
	// 		match backend::ds::lobby_runtime::ProxyProtocol::from_i32(
	// 			port.proxy_protocol,
	// 		) {
	// 			Some(
	// 				backend::ds::lobby_runtime::ProxyProtocol::Http
	// 				| backend::ds::lobby_runtime::ProxyProtocol::Https,
	// 			) => {
	// 				ports.push(path_proxied_port(lobby_id, region_id, port));
	// 			}
	// 			Some(
	// 				backend::ds::lobby_runtime::ProxyProtocol::Udp
	// 				| backend::ds::lobby_runtime::ProxyProtocol::Tcp
	// 				| backend::ds::lobby_runtime::ProxyProtocol::TcpTls,
	// 			)
	// 			| None => {}
	// 		}
	// 		ports
	// 	})
	// 	.collect::<GlobalResult<Vec<_>>>()?;

	// submit_job(&job_spec_json, Some(region_id.into()));

	// Get the region to dispatch in
	let region_res = op!([ctx] region_get {
		region_ids: vec![datacenter_id.into()],
	})
	.await?;
	let region = unwrap!(region_res.regions.first());

	// let region = region;
	let base_job: Job = serde_json::from_str::<nomad_client::models::Job>(&job_spec_json)?;

	// Modify the job spec
	let mut job = base_job;
	// let region = region;
	// Replace all job IDs with a placeholder value in order to create a
	// deterministic job spec.
	{
		let job_id: &str = "__PLACEHOLDER__";
		let job: &mut nomad_client::models::Job = &mut job;
		job.ID = Some(job_id.into());
		job.name = Some(job_id.into());
	};

	ensure_eq!(
		"batch",
		unwrap_ref!(job._type).as_str(),
		"only the batch job type is supported"
	);

	// Update the job's region
	job.region = Some(region.nomad_region.clone());
	job.datacenters = Some(vec![region.nomad_datacenter.clone()]);

	// Validate that the job is parameterized
	// TODO: clean up how stuff is put in here
	let parameters = unwrap!(job.parameterized_job.as_mut(), "job not parameterized");

	// Add run parameters
	parameters.meta_required = Some({
		let mut meta_required = parameters.meta_required.clone().unwrap_or_default();
		meta_required.push("job_run_id".into());
		meta_required
	});

	// Get task group
	let task_groups = unwrap!(job.task_groups.as_mut());
	ensure_eq!(1, task_groups.len(), "must have exactly 1 task group");
	let task_group = unwrap!(task_groups.first_mut());
	ensure_eq!(
		task_group.name.as_deref(),
		Some(RUN_MAIN_TASK_NAME),
		"must have main task group"
	);

	// Ensure has main task
	let main_task = unwrap!(
		task_group
			.tasks
			.iter_mut()
			.flatten()
			.find(|x| x.name.as_deref() == Some(RUN_MAIN_TASK_NAME)),
		"must have main task"
	);
	ensure!(
		main_task
			.lifecycle
			.as_ref()
			.map_or(true, |x| x.hook.is_none()),
		"main task must not have a lifecycle hook"
	);

	// Configure networks
	let networks = unwrap!(task_group.networks.as_mut());
	ensure_eq!(1, networks.len(), "must have exactly 1 network");
	let network = unwrap!(networks.first_mut());
	// Disable IPv6 DNS since Docker doesn't support IPv6 yet
	network.DNS = Some(Box::new(nomad_client::models::DnsConfig {
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
		..nomad_client::models::DnsConfig::new()
	}));

	// Disable rescheduling, since job-run doesn't support this at the moment
	task_group.reschedule_policy = Some(Box::new(nomad_client::models::ReschedulePolicy {
		attempts: Some(0),
		unlimited: Some(false),
		..nomad_client::models::ReschedulePolicy::new()
	}));

	// Disable restarts. Our Nomad monitoring workflow doesn't support restarts
	// at the moment.
	task_group.restart_policy = Some(Box::new(nomad_client::models::RestartPolicy {
		attempts: Some(0),
		// unlimited: Some(false),
		..nomad_client::models::RestartPolicy::new()
	}));

	// MARK: Cleanup task

	// Add cleanup task
	let tasks: &mut Vec<Task> = unwrap!(task_group.tasks.as_mut());
	tasks.push({
		Task {
			name: Some(RUN_CLEANUP_TASK_NAME.into()),
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
				CPU: Some(TASK_CLEANUP_CPU),
				memory_mb: Some(TASK_CLEANUP_MEMORY),
				..Resources::new()
			})),
			log_config: Some(Box::new(LogConfig {
				max_files: Some(4),
				max_file_size_mb: Some(2),
				disabled: Some(false),
			})),
			..Task::new()
		}
	});

	// Derive jobspec hash
	//
	// We serialize the JSON to a canonical string then take a SHA hash of the output.
	let job_cjson_str = match cjson::to_string(&job) {
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
		"job-{hash}:{region}",
		hash = &job_hash_str[0..12],
		region = region.name_id
	);
	{
		let job_id: &str = &job_id;
		let job: &mut nomad_client::models::Job = &mut job;
		job.ID = Some(job_id.into());
		job.name = Some(job_id.into());
	};

	// Submit the job
	tracing::info!("submitting job");

	// dbg!(
	// 	// &NEW_NOMAD_CONFIG,
	// 	&job_id,
	// 	nomad_client::models::JobRegisterRequest {
	// 		job: Some(Box::new(job.clone())),
	// 		..nomad_client::models::JobRegisterRequest::new()
	// 	},
	// 	Some(&region.nomad_region),
	// );
	// panic!();

	// pub struct Configuration {
	// 	pub base_path: String,
	// 	pub user_agent: Option<String>,
	// 	pub client: reqwest::Client,
	// 	pub basic_auth: Option<BasicAuth>,
	// 	pub oauth_access_token: Option<String>,
	// 	pub bearer_access_token: Option<String>,
	// 	pub api_key: Option<ApiKey>,
	// 	// TODO: take an oauth2 token source, similar to the go one
	// }

	// dbg!(
	// 	&NEW_NOMAD_CONFIG.base_path,
	// 	&NEW_NOMAD_CONFIG.user_agent,
	// 	&NEW_NOMAD_CONFIG.client,
	// 	&NEW_NOMAD_CONFIG.basic_auth,
	// 	&NEW_NOMAD_CONFIG.oauth_access_token,
	// 	&NEW_NOMAD_CONFIG.bearer_access_token,
	// 	&NEW_NOMAD_CONFIG.api_key,
	// );
	// panic!();

	let a = nomad_client::apis::jobs_api::post_job(
		&NEW_NOMAD_CONFIG,
		&job_id,
		nomad_client::models::JobRegisterRequest {
			job: Some(Box::new(job)),
			..nomad_client::models::JobRegisterRequest::new()
		},
		Some(&region.nomad_region),
		None,
		None,
		None,
	)
	.await?;
	dbg!(a);

	// let build_res = op!([ctx] build_get {
	// 	build_ids: vec![build_id.into()],
	// })
	// .await?;
	// let build = build_res.builds.first();
	// let build = unwrap_ref!(build);
	// let build_kind = unwrap!(backend::build::BuildKind::from_i32(build.kind));
	// let build_compression = unwrap!(backend::build::BuildCompression::from_i32(
	// 	build.compression
	// ));
	let upload_id_proto = unwrap!(build.upload_id);

	let upload_res = op!([ctx] upload_get {
		upload_ids: vec![upload_id_proto],
	})
	.await?;
	let upload = unwrap!(upload_res.uploads.first());

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

	let file_name = util_build::file_name(build_kind, build_compression);

	let mm_lobby_delivery_method = unwrap!(
		backend::cluster::BuildDeliveryMethod::from_i32(region.build_delivery_method),
		"invalid datacenter build delivery method"
	);
	let image_artifact_url = match mm_lobby_delivery_method {
		backend::cluster::BuildDeliveryMethod::S3Direct => {
			tracing::info!("using s3 direct delivery");

			let bucket = "bucket-build";

			// Build client
			let s3_client =
				s3_util::Client::from_env_opt(bucket, provider, s3_util::EndpointKind::External)
					.await?;

			let upload_id = unwrap_ref!(upload.upload_id).as_uuid();
			let presigned_req = s3_client
				.get_object()
				.bucket(s3_client.bucket())
				.key(format!("{upload_id}/{file_name}"))
				.presigned(
					s3_util::aws_sdk_s3::presigning::config::PresigningConfig::builder()
						.expires_in(std::time::Duration::from_secs(15 * 60))
						.build()?,
				)
				.await?;

			let addr = presigned_req.uri().clone();

			let addr_str = addr.to_string();
			tracing::info!(addr = %addr_str, "resolved artifact s3 presigned request");

			addr_str
		}
		backend::cluster::BuildDeliveryMethod::TrafficServer => {
			tracing::info!("using traffic server delivery");

			let region_id = unwrap_ref!(region.region_id).as_uuid();

			// Hash build so that the ATS server that we download the build from is always the same one. This
			// improves cache hit rates and reduces download times.
			let build_id = unwrap_ref!(build.build_id).as_uuid();
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
				// NOTE: region_id is just the old name for datacenter_id
				&region_id,
				serde_json::to_string(&cluster::types::PoolType::Ats)?,
				hash,
			)
			.await?;

			let upload_id = unwrap_ref!(upload.upload_id).as_uuid();
			let addr = format!(
				"http://{vlan_ip}:8080/s3-cache/{provider}/{namespace}-bucket-build/{upload_id}/{file_name}",
				vlan_ip = ats_vlan_ip,
				provider = heck::KebabCase::to_kebab_case(provider.as_str()),
				namespace = util::env::namespace(),
				upload_id = upload_id,
			);

			tracing::info!(%addr, "resolved artifact s3 url");

			addr
		}
	};

	let job_runner_binary_url = resolve_job_runner_binary_url(&ctx, region).await?;

	// MARK: Parameters

	let parameters: Vec<backend::job::Parameter> = vec![
		backend::job::Parameter {
			key: "job_runner_binary_url".into(),
			value: job_runner_binary_url,
		},
		backend::job::Parameter {
			key: "vector_socket_addr".into(),
			value: "127.0.0.1:5021".to_string(),
		},
		backend::job::Parameter {
			key: "image_artifact_url".into(),
			value: image_artifact_url.to_string(),
		},
		backend::job::Parameter {
			key: "root_user_enabled".into(),
			// TODO make table dynamic host, make reference so that we can find
			// other locations
			value: "0".into(),
		},
		backend::job::Parameter {
			key: "runner".into(),
			value: "dynamic_servers".into(),
		},
		backend::job::Parameter {
			key: "user_env".into(),
			// other locations
			value: unwrap!(serde_json::to_string(
				&ctx.environment
					.iter()
					.map(|(k, v)| (k.clone(), escape_go_template(v)))
					.collect::<HashMap<_, _>>(),
			)),
		},
	]
	.into_iter()
	// .chain(ctx.parameters.clone())
	// .chain(port_parameters)
	.collect();

	let job_params: Vec<(String, String)> = vec![("job_run_id".into(), server_id.to_string())];

	// MARK: Insert into db
	sql_execute!(
		[ctx]
		"
		INSERT INTO
			db_dynamic_servers.server_nomad (server_id)
		VALUES
			($1)
		",
		server_id,
	)
	.await?;

	// MARK: Dispatch
	let dispatch_res = nomad_client::apis::jobs_api::post_job_dispatch(
		&NEW_NOMAD_CONFIG,
		&job_id,
		nomad_client::models::JobDispatchRequest {
			job_id: Some(job_id.to_string()),
			payload: None,
			meta: Some(
				parameters
					.iter()
					.map(|p| (p.key.clone(), p.value.clone()))
					.chain(job_params.into_iter())
					.collect::<HashMap<String, String>>(),
			),
		},
		Some(&region.nomad_region),
		None,
		None,
		None,
	)
	.await;
	let nomad_dispatched_job_id: Option<String> = match dispatch_res {
		Ok(dispatch_res) => {
			// We will use the dispatched job ID to identify this allocation for the future. We can't use
			// eval ID, since that changes if we mutate the allocation (i.e. try to stop it).
			let nomad_dispatched_job_id = unwrap_ref!(dispatch_res.dispatched_job_id);
			GlobalResult::Ok(Some(nomad_dispatched_job_id.clone()))
		}
		Err(err) => {
			tracing::error!(?err, "failed to dispatch job");
			Ok(None)
		}
	}?;

	// MARK: Write to db after run
	sql_execute!(
		[ctx]
		"
		UPDATE
			db_dynamic_servers.server_nomad
		SET
			nomad_dispatched_job_id = $2
		WHERE
			server_id = $1
		",
		server_id,
		unwrap!(nomad_dispatched_job_id),
	)
	.await?;

	// Ok(job_id);

	// msg!([ctx] job_run::msg::create(run_id) {
	// 	run_id: Some(run_id.into()),
	// 	region_id: Some(region_id.into()),

	// 	job_spec_json: job_spec_json,
	// 	proxied_ports: proxied_ports,
	// 	..Default::default()
	// })
	// .await?;

	// Build response ports
	let network_ports = ctx
		.network_ports
		.iter()
		.map(|(port_label, port)| {
			GlobalResult::Ok((
				port_label.clone(),
				backend::ds::Port {
					internal_port: port.internal_port,
					public_hostname: None,
					public_port: None,
					routing: Some(match unwrap!(port.routing.clone()) {
						dynamic_servers::server_create::port::Routing::GameGuard(x) => {
							backend::ds::port::Routing::GameGuard(x)
						}
						dynamic_servers::server_create::port::Routing::Host(x) => {
							backend::ds::port::Routing::Host(x)
						}
					}),
				},
			))
		})
		.collect::<GlobalResult<HashMap<_, _>>>()?;

	Ok(dynamic_servers::server_create::Response {
		server: Some(backend::ds::Server {
			server_id: Some(server_id.into()),
			game_id: Some(game_id.into()),
			datacenter_id: Some(datacenter_id.into()),
			cluster_id: Some(cluster_id.into()),
			tags: ctx.tags.clone(),
			resources: Some(backend::ds::ServerResources {
				cpu_millicores: resources.cpu_millicores,
				memory_mib: resources.memory_mib,
			}),
			kill_timeout_ms: ctx.kill_timeout_ms,
			webhook_url: ctx.webhook_url.clone(),
			create_ts,
			start_ts: None,
			destroy_ts: None,
			args: ctx.args.clone(),
			environment: ctx.environment.clone(),
			image_id: ctx.image_id,
			network_mode: ctx.network_mode,
			network_ports,
		}),
	})
}

/// Determines if a Nomad job is dispatched from our run.
///
/// We use this when monitoring Nomad in order to determine which events to
/// pay attention to.
pub fn is_nomad_job_run(job_id: &str) -> bool {
	job_id.starts_with("job-") && job_id.contains("/dispatch-")
}

// Timeout from when `stop_job` is called and the kill signal is sent
pub const JOB_STOP_TIMEOUT: Duration = Duration::from_secs(30);

pub const TASK_CLEANUP_CPU: i32 = 50;

// Query Prometheus with:
//
// ```
// max(nomad_client_allocs_memory_max_usage{ns="prod",exported_job=~"job-.*",task="run-cleanup"}) / 1000 / 1000
// ```
//
// 13.5 MB baseline, 29 MB highest peak
pub const TASK_CLEANUP_MEMORY: i32 = 32;

pub const RUN_MAIN_TASK_NAME: &str = "main";
pub const RUN_CLEANUP_TASK_NAME: &str = "run-cleanup";

// dispatch, need alloc, nomad monitor stuff, lots of stuff here, means that
// jobs can't be destroyed, maybe by job id?

/// Generates a presigned URL for the job runner binary.
#[tracing::instrument]
async fn resolve_job_runner_binary_url(
	ctx: &OperationContext<dynamic_servers::server_create::Request>,
	region: &backend::region::Region,
) -> GlobalResult<String> {
	// Get provider
	let provider = s3_util::Provider::default()?;

	let file_name = std::env::var("JOB_RUNNER_BINARY_KEY")?;

	// Build URL
	let mm_lobby_delivery_method = unwrap!(
		backend::cluster::BuildDeliveryMethod::from_i32(region.build_delivery_method),
		"invalid datacenter build delivery method"
	);
	match mm_lobby_delivery_method {
		backend::cluster::BuildDeliveryMethod::S3Direct => {
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
		backend::cluster::BuildDeliveryMethod::TrafficServer => {
			tracing::info!("job runner using traffic server delivery");

			let region_id = unwrap_ref!(region.region_id).as_uuid();

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
				// NOTE: region_id is just the old name for datacenter_id
				&region_id,
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
