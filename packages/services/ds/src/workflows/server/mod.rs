use std::{
	collections::HashMap,
	hash::{DefaultHasher, Hasher},
	net::IpAddr,
	time::Duration,
};

use build::types::{BuildCompression, BuildKind};
use chirp_workflow::prelude::*;
use cluster::types::BuildDeliveryMethod;
use futures_util::FutureExt;
use rand::Rng;
use util::serde::AsHashableExt;

use crate::types::{
	GameGuardProtocol, NetworkMode, PortAuthorization, PortAuthorizationType, Routing,
	ServerLifecycle, ServerResources, ServerRuntime,
};

pub mod nomad;
pub mod pegboard;

// In ms, a small amount of time to separate the completion of the drain to the deletion of the
// cluster server. We want the drain to complete first.
const DRAIN_PADDING_MS: i64 = 10000;

// TODO: Restructure traefik to get rid of this
const TRAEFIK_GRACE_PERIOD: Duration = Duration::from_secs(2);

#[derive(Debug, Serialize, Deserialize)]
pub struct Input {
	pub server_id: Uuid,
	pub env_id: Uuid,
	pub datacenter_id: Uuid,
	pub cluster_id: Uuid,
	pub runtime: ServerRuntime,
	pub tags: HashMap<String, String>,
	pub resources: ServerResources,
	pub lifecycle: ServerLifecycle,
	pub image_id: Uuid,
	pub root_user_enabled: bool,
	pub args: Vec<String>,
	pub network_mode: NetworkMode,
	pub environment: HashMap<String, String>,
	pub network_ports: HashMap<String, Port>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
pub struct Port {
	// Null when using host networking since one is automatically assigned
	pub internal_port: Option<u16>,
	pub routing: Routing,
}

#[workflow]
pub async fn ds_server(ctx: &mut WorkflowCtx, input: &Input) -> GlobalResult<()> {
	let validation_res = ctx
		.activity(ValidateInput {
			env_id: input.env_id,
			datacenter_id: input.datacenter_id,
			tags: input.tags.as_hashable(),
			resources: input.resources.clone(),
			image_id: input.image_id,
			root_user_enabled: input.root_user_enabled,
			args: input.args.clone(),
			network_mode: input.network_mode,
			environment: input.environment.as_hashable(),
			network_ports: input.network_ports.as_hashable(),
		})
		.await?;

	if let Some(error_message) = validation_res {
		ctx.msg(Failed {
			message: error_message,
		})
		.tag("server_id", input.server_id)
		.send()
		.await?;

		// TODO(RVT-3928): return Ok(Err);
		return Ok(());
	}

	match input.runtime {
		ServerRuntime::Nomad => {
			ctx.workflow(nomad::Input {
				server_id: input.server_id,
				env_id: input.env_id,
				datacenter_id: input.datacenter_id,
				cluster_id: input.cluster_id,
				tags: input.tags.clone(),
				resources: input.resources.clone(),
				lifecycle: input.lifecycle.clone(),
				image_id: input.image_id,
				root_user_enabled: input.root_user_enabled,
				args: input.args.clone(),
				network_mode: input.network_mode,
				environment: input.environment.clone(),
				network_ports: input.network_ports.clone(),
			})
			.output()
			.await
		}
		ServerRuntime::Pegboard => {
			ctx.workflow(pegboard::Input {
				server_id: input.server_id,
				env_id: input.env_id,
				datacenter_id: input.datacenter_id,
				cluster_id: input.cluster_id,
				tags: input.tags.clone(),
				resources: input.resources.clone(),
				lifecycle: input.lifecycle.clone(),
				image_id: input.image_id,
				root_user_enabled: input.root_user_enabled,
				args: input.args.clone(),
				network_mode: input.network_mode,
				environment: input.environment.clone(),
				network_ports: input.network_ports.clone(),
			})
			.output()
			.await
		}
	}
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
struct ValidateInput {
	datacenter_id: Uuid,
	env_id: Uuid,
	tags: util::serde::HashableMap<String, String>,
	resources: ServerResources,
	image_id: Uuid,
	root_user_enabled: bool,
	args: Vec<String>,
	network_mode: NetworkMode,
	environment: util::serde::HashableMap<String, String>,
	network_ports: util::serde::HashableMap<String, Port>,
}

// TODO: Redo once a solid global error solution is established so we dont have to have validation all in one
// place.
#[activity(Validate)]
async fn validate(ctx: &ActivityCtx, input: &ValidateInput) -> GlobalResult<Option<String>> {
	let (tier_res, upload_res, game_config_res) = tokio::try_join!(
		async {
			let datacenters_res = ctx
				.op(cluster::ops::datacenter::get::Input {
					datacenter_ids: vec![input.datacenter_id],
				})
				.await?;

			let Some(datacenter) = datacenters_res.datacenters.first() else {
				return GlobalResult::Ok(None);
			};

			let tier_res = ctx
				.op(tier::ops::list::Input {
					datacenter_ids: vec![datacenter.datacenter_id],
					pegboard: true,
				})
				.await?;
			let tier_dc = unwrap!(tier_res.datacenters.first());

			// Find any tier that has more CPU and memory than the requested resources
			Ok(Some(tier_dc.tiers.iter().any(|t| {
				t.cpu_millicores >= input.resources.cpu_millicores
					&& t.memory >= input.resources.memory_mib
			})))
		},
		async {
			let builds_res = ctx
				.op(build::ops::get::Input {
					build_ids: vec![input.image_id],
				})
				.await?;

			let Some(build) = builds_res.builds.first() else {
				return Ok(None);
			};

			let uploads_res = op!([ctx] upload_get {
				upload_ids: vec![build.upload_id.into()],
			})
			.await?;

			Ok(Some(
				unwrap!(uploads_res.uploads.first()).complete_ts.is_some(),
			))
		},
		async {
			let games_res = op!([ctx] game_resolve_namespace_id {
				namespace_ids: vec![input.env_id.into()],
			})
			.await?;

			let Some(game) = games_res.games.first() else {
				return Ok(None);
			};

			let game_config_res = ctx
				.op(crate::ops::game_config::get::Input {
					game_ids: vec![unwrap!(game.game_id).into()],
				})
				.await?;

			Ok(Some(unwrap!(game_config_res.game_configs.first()).clone()))
		}
	)?;

	let Some(has_tier) = tier_res else {
		return Ok(Some("Region not found.".into()));
	};

	if !has_tier {
		return Ok(Some("Too many resources allocated.".into()));
	}

	let Some(upload_complete) = upload_res else {
		return Ok(Some("Build not found.".into()));
	};

	if !upload_complete {
		return Ok(Some("Build upload not complete.".into()));
	}

	let Some(game_config) = game_config_res else {
		return Ok(Some("Environment not found.".into()));
	};

	if matches!(input.network_mode, NetworkMode::Host) && !game_config.host_networking_enabled {
		return Ok(Some("Host networking is not enabled for this game.".into()));
	}

	if input.root_user_enabled && !game_config.root_user_enabled {
		return Ok(Some(
			"Docker root user is not enabled for this game.".into(),
		));
	}

	if input.tags.len() > 64 {
		return Ok(Some("Too many tags (max 64).".into()));
	}

	for (k, v) in &input.tags {
		if k.is_empty() {
			return Ok(Some("tags[]: Tag label cannot be empty.".into()));
		}
		if k.len() > 256 {
			return Ok(Some(format!(
				"tags[{:?}]: Tag label too large (max 256 bytes).",
				&k[..256]
			)));
		}
		if v.is_empty() {
			return Ok(Some(format!("tags[{k:?}]: Tag value cannot be empty.",)));
		}
		if v.len() > 1024 {
			return Ok(Some(format!(
				"tags[{k:?}]: Tag value too large (max 1024 bytes)."
			)));
		}
	}

	if input.args.len() > 64 {
		return Ok(Some("Too many arguments (max 64).".into()));
	}

	for (i, arg) in input.args.iter().enumerate() {
		if arg.len() > 256 {
			return Ok(Some(format!(
				"runtime.args[{i}]: Argument too large (max 256 bytes)."
			)));
		}
	}

	if input.environment.len() > 64 {
		return Ok(Some("Too many environment variables (max 64).".into()));
	}

	for (k, v) in &input.environment {
		if k.len() > 256 {
			return Ok(Some(format!(
				"runtime.environment[{:?}]: Key too large (max 256 bytes).",
				&k[..256]
			)));
		}
		if v.len() > 1024 {
			return Ok(Some(format!(
				"runtime.environment[{k:?}]: Value too large (max 1024 bytes)."
			)));
		}
	}

	if input.network_ports.len() > 64 {
		return Ok(Some("Too many ports (max 64).".into()));
	}

	for (name, port) in &input.network_ports {
		if name.len() > 256 {
			return Ok(Some(format!(
				"runtime.ports[{:?}]: Port name too large (max 256 bytes).",
				&name[..256]
			)));
		}

		match &port.routing {
			Routing::GameGuard { authorization, .. } => match authorization {
				PortAuthorization::Bearer(token) => {
					if token.len() > 1024 {
						return Ok(Some(format!(
								"runtime.ports[{name:?}].routing.guard.authorization.bearer: Bearer authorization too large (max 1024 bytes).",
							)));
					}
				}
				PortAuthorization::Query(parameter, value) => {
					if parameter.len() > 128 {
						return Ok(Some(format!(
								"runtime.ports[{name:?}].routing.guard.authorization.query: Query parameter too large (max 128 bytes).",
							)));
					}
					if value.len() > 1024 {
						return Ok(Some(format!(
								"runtime.ports[{name:?}].routing.guard.authorization.query: Query value too large (max 1024 bytes).",
							)));
					}
				}
				PortAuthorization::None => {}
			},
			Routing::Host { .. } => {}
		}
	}

	Ok(None)
}

#[derive(Clone, Debug, Default)]
struct GameGuardUnnest {
	pub port_names: Vec<String>,
	pub port_numbers: Vec<Option<i32>>,
	pub protocols: Vec<i32>,
	pub gg_ports: Vec<i32>,
}

#[derive(Clone, Debug, Default)]
struct GameGuardAuthUnnest {
	pub port_names: Vec<String>,
	pub port_auth_types: Vec<i32>,
	pub port_auth_keys: Vec<Option<String>>,
	pub port_auth_values: Vec<String>,
}

#[derive(Clone, Debug, Default)]
struct HostUnnest {
	pub port_names: Vec<String>,
	pub port_numbers: Vec<Option<i32>>,
	pub protocols: Vec<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
struct InsertDbInput {
	server_id: Uuid,
	env_id: Uuid,
	datacenter_id: Uuid,
	cluster_id: Uuid,
	tags: util::serde::HashableMap<String, String>,
	resources: ServerResources,
	lifecycle: ServerLifecycle,
	image_id: Uuid,
	args: Vec<String>,
	network_mode: NetworkMode,
	environment: util::serde::HashableMap<String, String>,
	network_ports: util::serde::HashableMap<String, Port>,
}

#[activity(InsertDb)]
async fn insert_db(ctx: &ActivityCtx, input: &InsertDbInput) -> GlobalResult<()> {
	let mut gg_unnest = GameGuardUnnest::default();
	let mut gg_auth_unnest = GameGuardAuthUnnest::default();
	let mut host_unnest = HostUnnest::default();

	for (name, port) in input.network_ports.iter() {
		match port.routing {
			Routing::GameGuard {
				protocol,
				ref authorization,
			} => {
				gg_unnest.port_names.push(name.clone());
				gg_unnest
					.port_numbers
					.push(port.internal_port.map(|x| x as i32));
				gg_unnest.protocols.push(protocol as i32);
				gg_unnest
					.gg_ports
					.push(choose_ingress_port(ctx, protocol).await? as i32);

				match authorization {
					PortAuthorization::None => {}
					PortAuthorization::Bearer(token) => {
						gg_auth_unnest.port_names.push(name.clone());
						gg_auth_unnest
							.port_auth_types
							.push(PortAuthorizationType::Bearer as i32);
						gg_auth_unnest.port_auth_keys.push(None);
						gg_auth_unnest.port_auth_values.push(token.clone());
					}
					PortAuthorization::Query(key, value) => {
						gg_auth_unnest.port_names.push(name.clone());
						gg_auth_unnest
							.port_auth_types
							.push(PortAuthorizationType::Query as i32);
						gg_auth_unnest.port_auth_keys.push(Some(key.clone()));
						gg_auth_unnest.port_auth_values.push(value.clone());
					}
				}
			}
			Routing::Host { protocol } => {
				host_unnest.port_names.push(name.clone());
				host_unnest
					.port_numbers
					.push(port.internal_port.map(|x| x as i32));
				host_unnest.protocols.push(protocol as i32);
			}
		};
	}

	// Run in a transaction for retryability
	rivet_pools::utils::crdb::tx(&ctx.crdb().await?, |tx| {
		let ctx = ctx.clone();
		let input = input.clone();
		let host_unnest = host_unnest.clone();
		let gg_unnest = gg_unnest.clone();
		let gg_auth_unnest = gg_auth_unnest.clone();

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
							lifecycle_kill_timeout_ms,
							lifecycle_durable,
							create_ts,
							image_id,
							args,
							network_mode,
							environment
						)
						VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
						RETURNING 1
					),
					host_port AS (
						INSERT INTO db_ds.server_ports_host (
							server_id,
							port_name,
							port_number,
							protocol
						)
						SELECT $1, t.*
						FROM unnest($15, $16, $17) AS t(port_name, port_number, protocol)
						RETURNING 1
					),
					gg_port AS (
						INSERT INTO db_ds.server_ports_gg (
							server_id,
							port_name,
							port_number,
							protocol,
							gg_port
						)
						SELECT $1, t.*
						FROM unnest($18, $19, $20, $21) AS t(port_name, port_number, protocol, gg_port)
						RETURNING 1
					),
					gg_port_auth AS (
						INSERT INTO db_ds.server_ports_gg_auth (
							server_id,
							port_name,
							auth_type,
							key,
							value
						)
						SELECT $1, t.*
						FROM unnest($22, $23, $24, $25) AS t(port_name, auth_type, auth_key, auth_value)
						RETURNING 1
					)
				SELECT 1
				",
				input.server_id,
				input.env_id,
				input.datacenter_id,
				input.cluster_id,
				serde_json::to_string(&input.tags)?, // 5
				input.resources.cpu_millicores as i32,
				input.resources.memory_mib as i32,
				input.lifecycle.kill_timeout_ms,
				input.lifecycle.durable,
				ctx.ts(), // 10
				input.image_id,
				&input.args,
				input.network_mode as i32,
				serde_json::to_string(&input.environment)?,
				host_unnest.port_names, // 15
				host_unnest.port_numbers,
				host_unnest.protocols,
				gg_unnest.port_names,
				gg_unnest.port_numbers,
				gg_unnest.protocols, // 20
				gg_unnest.gg_ports,
				gg_auth_unnest.port_names,
				gg_auth_unnest.port_auth_types,
				gg_auth_unnest.port_auth_keys, // 20
				gg_auth_unnest.port_auth_values,
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
		.purge("ds_proxied_ports", [input.datacenter_id])
		.await?;

	Ok(())
}

#[derive(Debug, Serialize, Deserialize, Hash)]
struct GetServerMetaInput {
	env_id: Uuid,
	datacenter_id: Uuid,
	image_id: Uuid,
}

#[derive(Debug, Serialize, Deserialize, Hash)]
struct GetServerMetaOutput {
	project_id: Uuid,
	project_slug: String,
	env_slug: String,
	build_upload_id: Uuid,
	build_file_name: String,
	build_kind: BuildKind,
	build_compression: BuildCompression,
	dc_name_id: String,
	dc_display_name: String,
	dc_build_delivery_method: BuildDeliveryMethod,
}

#[activity(GetServerMeta)]
async fn get_server_meta(
	ctx: &ActivityCtx,
	input: &GetServerMetaInput,
) -> GlobalResult<GetServerMetaOutput> {
	// Validate build exists and belongs to this game
	let (env_res, build_res, dc_res) = tokio::try_join!(
		op!([ctx] game_namespace_get {
			namespace_ids: vec![input.env_id.into()],
		}),
		ctx.op(build::ops::get::Input {
			build_ids: vec![input.image_id],
		}),
		ctx.op(cluster::ops::datacenter::get::Input {
			datacenter_ids: vec![input.datacenter_id],
		})
	)?;
	let env = unwrap_with!(env_res.namespaces.first(), ENVIRONMENT_NOT_FOUND);
	let build = unwrap_with!(build_res.builds.first(), BUILD_NOT_FOUND);

	let dc = unwrap!(dc_res.datacenters.first());

	// Lookup project
	let project_id = unwrap!(env.game_id).as_uuid();
	let projects_res = op!([ctx] game_get {
		game_ids: vec![project_id.into()],
	}).await?;
	let project = unwrap!(projects_res.games.first());

	Ok(GetServerMetaOutput {
		project_id,
		project_slug: project.name_id.clone(),
		env_slug: env.name_id.clone(),
		build_upload_id: build.upload_id,
		build_file_name: build::utils::file_name(build.kind, build.compression),
		build_kind: build.kind,
		build_compression: build.compression,
		dc_name_id: dc.name_id.clone(),
		dc_display_name: dc.display_name.clone(),
		dc_build_delivery_method: dc.build_delivery_method,
	})
}

#[derive(Debug, Serialize, Deserialize, Hash)]
struct SetConnectableInput {
	server_id: Uuid,
}

#[activity(SetConnectable)]
async fn set_connectable(ctx: &ActivityCtx, input: &SetConnectableInput) -> GlobalResult<bool> {
	let res = sql_execute!(
		[ctx]
		"
		UPDATE db_ds.servers
		SET connectable_ts = $2
		WHERE
			server_id = $1 AND
			connectable_ts IS NULL
		",
		input.server_id,
		util::timestamp::now(),
	)
	.await?;

	Ok(res.rows_affected() > 0)
}

#[derive(Debug, Serialize, Deserialize, Hash)]
struct UpdateImageInput {
	server_id: Uuid,
	image_id: Uuid,
}

#[activity(UpdateImage)]
async fn update_image(ctx: &ActivityCtx, input: &UpdateImageInput) -> GlobalResult<()> {
	sql_execute!(
		[ctx]
		"
		UPDATE db_ds.servers
		SET image_id = $2
		WHERE server_id = $1
		",
		input.server_id,
		input.image_id,
	)
	.await?;

	Ok(())
}

#[message("ds_server_create_complete")]
pub struct CreateComplete {}

#[message("ds_server_failed")]
pub struct Failed {
	pub message: String,
}

#[message("ds_server_ready")]
pub struct Ready {}

#[signal("ds_server_destroy")]
pub struct Destroy {
	pub override_kill_timeout_ms: Option<i64>,
}

#[message("ds_server_destroy_started")]
pub struct DestroyStarted {}

#[message("ds_server_destroy_complete")]
pub struct DestroyComplete {}

#[signal("ds_server_upgrade")]
pub struct Upgrade {
	pub image_id: Uuid,
}

#[message("ds_server_upgrade_started")]
pub struct UpgradeStarted {}

#[message("ds_server_upgrade_complete")]
pub struct UpgradeComplete {}

#[signal("ds_server_drain")]
pub struct Drain {
	pub drain_timeout: i64,
}

#[signal("ds_server_undrain")]
pub struct Undrain {}

#[rustfmt::skip]
join_signal!(DrainState {
	Undrain,
	Destroy,
});

/// Generates a presigned URL for the build image.
async fn resolve_image_artifact_url(
	ctx: &ActivityCtx,
	datacenter_id: Uuid,
	build_file_name: String,
	build_delivery_method: BuildDeliveryMethod,
	build_id: Uuid,
	upload_id: Uuid,
) -> GlobalResult<String> {
	// Build URL
	match build_delivery_method {
		BuildDeliveryMethod::S3Direct => {
			tracing::debug!("using s3 direct delivery");

			// Build client
			let s3_client = s3_util::Client::with_bucket_and_endpoint(
				ctx.config(),
				"bucket-build",
				s3_util::EndpointKind::EdgeInternal,
			)
			.await?;

			let presigned_req = s3_client
				.get_object()
				.bucket(s3_client.bucket())
				.key(format!("{upload_id}/{build_file_name}"))
				.presigned(
					s3_util::aws_sdk_s3::presigning::PresigningConfig::builder()
						.expires_in(std::time::Duration::from_secs(15 * 60))
						.build()?,
				)
				.await?;

			let addr_str = presigned_req.uri().to_string();
			tracing::debug!(addr = %addr_str, "resolved artifact s3 presigned request");

			Ok(addr_str)
		}
		BuildDeliveryMethod::TrafficServer => {
			tracing::debug!("using traffic server delivery");

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
						pool_type = $2 AND
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
				cluster::types::PoolType::Ats as i32,
				hash,
			)
			.await?;

			let addr = format!(
				"http://{vlan_ip}:8080/s3-cache/{namespace}-bucket-build/{upload_id}/{build_file_name}",
				vlan_ip = ats_vlan_ip,
				namespace = ctx.config().server()?.rivet.namespace,
				upload_id = upload_id,
			);

			tracing::debug!(%addr, "resolved artifact s3 url");

			Ok(addr)
		}
	}
}

/// Choose which port to assign for a job's ingress port.
/// This is required because TCP and UDP do not have a `Host` header and thus cannot be re-routed by hostname.
///
/// If not provided by `ProxiedPort`, then:
/// - HTTP: 80
/// - HTTPS: 443
/// - TCP/TLS: random
/// - UDP: random
async fn choose_ingress_port(ctx: &ActivityCtx, protocol: GameGuardProtocol) -> GlobalResult<u16> {
	let gg_config = &ctx.config().server()?.rivet.guard;

	match protocol {
		GameGuardProtocol::Http => Ok(80),
		GameGuardProtocol::Https => Ok(443),
		GameGuardProtocol::Tcp | GameGuardProtocol::TcpTls => {
			bind_with_retries(
				ctx,
				protocol,
				gg_config.min_ingress_port_tcp()..=gg_config.max_ingress_port_tcp(),
			)
			.await
		}
		GameGuardProtocol::Udp => {
			bind_with_retries(
				ctx,
				protocol,
				gg_config.min_ingress_port_udp()..=gg_config.max_ingress_port_udp(),
			)
			.await
		}
	}
}

// TODO: Use the same algorithm for picking ports as vlan ip in cluster or ports in the mb manager
/// This is very poorly written for TCP & UDP ports and may bite us in the ass
/// some day. See https://linear.app/rivet-gg/issue/RVT-1799
async fn bind_with_retries(
	ctx: &ActivityCtx,
	proxy_protocol: GameGuardProtocol,
	range: std::ops::RangeInclusive<u16>,
) -> GlobalResult<u16> {
	let mut attempts = 3;

	// Try to bind to a random port, verifying that it is not already bound
	loop {
		if attempts == 0 {
			bail!("failed all attempts to bind to unique port");
		}
		attempts -= 1;

		let port = rand::thread_rng().gen_range(range.clone());

		let (already_exists,) = sql_fetch_one!(
			[ctx, (bool,)]
			"
			SELECT EXISTS(
				SELECT 1
				FROM db_ds.servers AS s
				JOIN db_ds.server_ports_gg AS p
				ON s.server_id = p.server_id
				WHERE
					s.destroy_ts IS NULL AND
					p.gg_port = $1 AND
					p.protocol = $2
			)
			",
			port as i32,
			proxy_protocol as i32,
		)
		.await?;

		if !already_exists {
			break Ok(port);
		}

		tracing::debug!(?port, ?attempts, "port collision, retrying");
	}
}
