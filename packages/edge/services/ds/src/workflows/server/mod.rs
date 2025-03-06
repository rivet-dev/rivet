use std::collections::HashMap;

use build::types::{BuildCompression, BuildKind};
use chirp_workflow::prelude::*;
use cluster::types::BuildDeliveryMethod;
use fdb_util::FormalKey;
use foundationdb as fdb;
use sqlite_util::SqlitePoolExt;
use util::serde::AsHashableExt;
use sqlx::Acquire;

use crate::{
	keys,
	types::{GameGuardProtocol, NetworkMode, Routing, ServerLifecycle, ServerResources},
};

pub mod pegboard;

// In ms, a small amount of time to separate the completion of the drain to the deletion of the
// cluster server. We want the drain to complete first.
const DRAIN_PADDING_MS: i64 = 10000;
/// Time to delay an actor from rescheduling after a rescheduling failure.
const BASE_RETRY_TIMEOUT_MS: usize = 2000;

#[derive(Debug, Serialize, Deserialize)]
pub struct Input {
	pub server_id: Uuid,
	pub env_id: Uuid,
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

	let network_ports = ctx
		.activity(DisableTlsPortsInput {
			network_ports: input.network_ports.as_hashable(),
		})
		.await?;

	ctx.workflow(pegboard::Input {
		server_id: input.server_id,
		env_id: input.env_id,
		tags: input.tags.clone(),
		resources: input.resources.clone(),
		lifecycle: input.lifecycle.clone(),
		image_id: input.image_id,
		root_user_enabled: input.root_user_enabled,
		args: input.args.clone(),
		network_mode: input.network_mode,
		environment: input.environment.clone(),
		network_ports: network_ports.into_iter().collect(),
	})
	.output()
	.await
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
struct ValidateInput {
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
	let dc_id = ctx.config().server()?.rivet.edge()?.datacenter_id;

	let (has_tier, upload_res, game_config_res) = tokio::try_join!(
		async {
			let tier_res = ctx
				.op(tier::ops::list::Input {
					datacenter_ids: vec![dc_id],
					pegboard: true,
				})
				.await?;
			let tier_dc = unwrap!(tier_res.datacenters.first());

			// Find any tier that has more CPU and memory than the requested resources
			GlobalResult::Ok(tier_dc.tiers.iter().any(|t| {
				t.cpu_millicores >= input.resources.cpu_millicores
					&& t.memory >= input.resources.memory_mib
			}))
		},
		async {
			let builds_res = ctx
				.op(build::ops::get::Input {
					build_ids: vec![input.image_id],
				})
				.await?;

			let Some(build) = builds_res.builds.into_iter().next() else {
				return Ok(None);
			};

			let uploads_res = op!([ctx] upload_get {
				upload_ids: vec![build.upload_id.into()],
			})
			.await?;

			Ok(Some((
				build,
				unwrap!(uploads_res.uploads.first()).complete_ts.is_some(),
			)))
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

	if !has_tier {
		return Ok(Some("Too many resources allocated.".into()));
	}

	let Some((build, upload_complete)) = upload_res else {
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

	if input.tags.len() > 8 {
		return Ok(Some("Too many tags (max 8).".into()));
	}

	for (k, v) in &input.tags {
		if k.is_empty() {
			return Ok(Some("tags[]: Tag label cannot be empty.".into()));
		}
		if k.len() > 32 {
			return Ok(Some(format!(
				"tags[{:?}]: Tag label too large (max 32 bytes).",
				util::safe_slice(k, 0, 32),
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
				util::safe_slice(k, 0, 256),
			)));
		}
		if v.len() > 1024 {
			return Ok(Some(format!(
				"runtime.environment[{k:?}]: Value too large (max 1024 bytes)."
			)));
		}
	}

	if input.network_ports.len() > 8 {
		return Ok(Some("Too many ports (max 8).".into()));
	}

	for (name, port) in &input.network_ports {
		if name.len() > 16 {
			return Ok(Some(format!(
				"runtime.ports[{:?}]: Port name too large (max 16 bytes).",
				util::safe_slice(name, 0, 16),
			)));
		}

		match input.network_mode {
			NetworkMode::Bridge => {
				// NOTE: Temporary validation until we implement bridge networking for isolates
				if let BuildKind::JavaScript = build.kind {
					if port.internal_port.is_some() {
						return Ok(Some(format!(
							"runtime.ports[{name:?}].internal_port: Must be null when `network.mode` = \"bridge\" and using a JS build.",
						)));
					}
				}
			}
			NetworkMode::Host => {
				if port.internal_port.is_some() {
					return Ok(Some(format!(
						"runtime.ports[{name:?}].internal_port: Must be null when `network.mode` = \"host\".",
					)));
				}
			}
		}
	}

	Ok(None)
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
struct DisableTlsPortsInput {
	network_ports: util::serde::HashableMap<String, Port>,
}

/// If TLS is not enabled in the cluster, we downgrade all protocols to the non-TLS equivalents.
/// This allows developers to develop locally with the same code they would use in production.
#[activity(DisableTlsPorts)]
async fn disable_tls_ports(
	ctx: &ActivityCtx,
	input: &DisableTlsPortsInput,
) -> GlobalResult<util::serde::HashableMap<String, Port>> {
	if ctx.config().server()?.rivet.guard.tls_enabled() {
		// Do nothing
		Ok(input.network_ports.clone())
	} else {
		// Downgrade all TLS protocols to non-TLS protocols
		let network_ports = input
			.network_ports
			.clone()
			.into_iter()
			.map(|(k, p)| {
				(
					k,
					Port {
						internal_port: p.internal_port,
						routing: match p.routing {
							Routing::GameGuard { protocol } => Routing::GameGuard {
								protocol: match protocol {
									GameGuardProtocol::Https => GameGuardProtocol::Http,
									GameGuardProtocol::TcpTls => GameGuardProtocol::Tcp,
									x @ (GameGuardProtocol::Http
									| GameGuardProtocol::Tcp
									| GameGuardProtocol::Udp) => x,
								},
							},
							x @ Routing::Host { .. } => x,
						},
					},
				)
			})
			.collect();

		Ok(network_ports)
	}
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
struct InsertDbInput {
	server_id: Uuid,
	env_id: Uuid,
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
async fn insert_db(ctx: &ActivityCtx, input: &InsertDbInput) -> GlobalResult<i64> {
	let mut conn = ctx.sqlite().await?.conn().await?;
	let mut tx = conn.begin().await?;
	let create_ts = ctx.ts();

	sql_execute!(
		[ctx, @tx &mut tx]
		"
		INSERT INTO state (
			env_id,
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
		VALUES (?, jsonb(?), ?, ?, ?, ?, ?, ?, jsonb(?), ?, jsonb(?))
		",
		input.env_id,
		serde_json::to_string(&input.tags)?,
		input.resources.cpu_millicores as i32,
		input.resources.memory_mib as i32,
		input.lifecycle.kill_timeout_ms,
		input.lifecycle.durable,
		create_ts,
		input.image_id,
		serde_json::to_string(&input.args)?,
		input.network_mode as i32,
		serde_json::to_string(&input.environment)?,
	)
	.await?;

	sql_execute!(
		[ctx, @tx &mut tx]
		"
		INSERT INTO pegboard (client_id, client_wan_hostname)
		VALUES (NULL, NULL)
		",
	)
	.await?;

	// Count up ports per protocol
	let mut port_counts = Vec::new();
	for (_, port) in &input.network_ports {
		match port.routing {
			Routing::GameGuard {
				protocol:
					protocol @ (GameGuardProtocol::Tcp
					| GameGuardProtocol::TcpTls
					| GameGuardProtocol::Udp),
			} => {
				if let Some((_, count)) = port_counts.iter_mut().find(|(p, _)| &protocol == p) {
					*count += 1;
				} else {
					port_counts.push((protocol, 1));
				}
			}
			_ => {}
		}
	}

	// Choose which port to assign for a job's ingress port.
	// This is required because TCP and UDP do not have a `Host` header and thus cannot be re-routed by hostname.
	//
	// If not provided by `ProxiedPort`, then:
	// - HTTP: 80
	// - HTTPS: 443
	// - TCP/TLS: random
	// - UDP: random
	let ingress_ports_res = ctx
		.op(crate::ops::server::allocate_ingress_ports::Input {
			server_id: input.server_id,
			ports: port_counts,
		})
		.await?;
	let mut ingress_ports = ingress_ports_res
		.ports
		.into_iter()
		.map(|(protocol, ports)| (protocol, ports.into_iter()))
		.collect::<Vec<_>>();

	let gg_config = &ctx.config().server()?.rivet.guard;
	for (name, port) in input.network_ports.iter() {
		match port.routing {
			Routing::GameGuard { protocol } => {
				sql_execute!(
					[ctx, @tx &mut tx]
					"
					INSERT INTO server_ports_ingress (
						port_name,
						port_number,
						protocol,
						ingress_port_number
					)
					VALUES (?, ?, ?, ?)
					",
					name,
					port.internal_port.map(|x| x as i32),
					protocol as i32,
					match protocol {
						GameGuardProtocol::Http => gg_config.http_port(),
						GameGuardProtocol::Https => gg_config.https_port(),
						GameGuardProtocol::Tcp | GameGuardProtocol::TcpTls | GameGuardProtocol::Udp => {
							let (_, ports_iter) = unwrap!(
								ingress_ports.iter_mut().find(|(p, _)| &protocol == p)
							);
							unwrap!(ports_iter.next(), "missing ingress port")
						},
					} as i32,
				)
				.await?;
			}
			Routing::Host { protocol } => {
				sql_execute!(
					[ctx, @tx &mut tx]
					"
					INSERT INTO server_ports_host (
						port_name,
						port_number,
						protocol
					)
					VALUES (?, ?, ?)
					",
					name,
					port.internal_port.map(|x| x as i32),
					protocol as i32,
				)
				.await?;
			}
		};
	}

	tx.commit().await?;

	Ok(create_ts)
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
struct PopulateFdbIdxInput {
	server_id: Uuid,
	env_id: Uuid,
	tags: util::serde::HashableMap<String, String>,
	create_ts: i64,
}

#[activity(PopulateFdbIdx)]
async fn populate_fdb_idx(ctx: &ActivityCtx, input: &PopulateFdbIdxInput) -> GlobalResult<()> {
	ctx.fdb()
		.await?
		.run(|tx, _mc| async move {
			let create_ts_key = keys::server::CreateTsKey::new(input.server_id);

			tx.set(
				&keys::subspace().pack(&create_ts_key),
				&create_ts_key
					.serialize(input.create_ts)
					.map_err(|x| fdb::FdbBindingError::CustomError(x.into()))?,
			);

			let server_key =
				keys::env::ServerKey::new(input.env_id, input.create_ts, input.server_id);
			let data = keys::env::ServerKeyData {
				is_destroyed: false,
				tags: input.tags.clone().into_iter().collect(),
			};

			tx.set(
				&keys::subspace().pack(&server_key),
				&server_key
					.serialize(data)
					.map_err(|x| fdb::FdbBindingError::CustomError(x.into()))?,
			);

			Ok(())
		})
		.await?;

	Ok(())
}

#[derive(Debug, Serialize, Deserialize, Hash)]
struct GetServerMetaInput {
	env_id: Uuid,
	image_id: Uuid,
}

#[derive(Clone, Debug, Serialize, Deserialize, Hash)]
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
	let dc_id = ctx.config().server()?.rivet.edge()?.datacenter_id;

	// Validate build exists and belongs to this game
	let (env_res, build_res, dc_res) = tokio::try_join!(
		op!([ctx] game_namespace_get {
			namespace_ids: vec![input.env_id.into()],
		}),
		ctx.op(build::ops::get::Input {
			build_ids: vec![input.image_id],
		}),
		ctx.op(cluster::ops::datacenter::get::Input {
			datacenter_ids: vec![dc_id],
		})
	)?;
	let env = unwrap_with!(env_res.namespaces.first(), ENVIRONMENT_NOT_FOUND);
	let build = unwrap_with!(build_res.builds.first(), BUILD_NOT_FOUND);
	let dc = unwrap!(dc_res.datacenters.first());

	// Lookup project
	let project_id = unwrap!(env.game_id).as_uuid();
	let projects_res = op!([ctx] game_get {
		game_ids: vec![project_id.into()],
	})
	.await?;
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
	connectable: bool,
}

#[activity(SetConnectable)]
async fn set_connectable(ctx: &ActivityCtx, input: &SetConnectableInput) -> GlobalResult<bool> {
	let pool = ctx.sqlite().await?;

	let res = sql_execute!(
		[ctx, pool]
		"
		UPDATE state
		SET connectable_ts = ?
		WHERE
			CASE WHEN ?
			THEN connectable_ts IS NULL
			ELSE connectable_ts IS NOT NULL
			END
		",
		input.connectable.then(util::timestamp::now),
		input.connectable,
	)
	.await?;

	Ok(res.rows_affected() > 0)
}

#[derive(Debug, Serialize, Deserialize, Hash)]
struct UpdateImageInput {
	image_id: Uuid,
}

#[activity(UpdateImage)]
async fn update_image(ctx: &ActivityCtx, input: &UpdateImageInput) -> GlobalResult<()> {
	sql_execute!(
		[ctx]
		"
		UPDATE state
		SET image_id = ?
		",
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
