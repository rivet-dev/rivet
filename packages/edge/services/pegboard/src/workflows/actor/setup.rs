use build::types::{BuildCompression, BuildKind};
use chirp_workflow::prelude::*;
use cluster::types::BuildDeliveryMethod;
use fdb_util::FormalKey;
use foundationdb as fdb;
use sqlx::Acquire;

use super::{Input, Port};
use crate::{
	keys, protocol,
	types::{ActorLifecycle, ActorResources, GameGuardProtocol, NetworkMode, Routing},
};

#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
pub struct ValidateInput {
	pub env_id: Uuid,
	pub tags: util::serde::HashableMap<String, String>,
	pub resources: ActorResources,
	pub image_id: Uuid,
	pub root_user_enabled: bool,
	pub args: Vec<String>,
	pub network_mode: NetworkMode,
	pub environment: util::serde::HashableMap<String, String>,
	pub network_ports: util::serde::HashableMap<String, Port>,
}

// TODO: Redo once a solid global error solution is established so we dont have to have validation all in one
// place.
#[activity(Validate)]
pub async fn validate(ctx: &ActivityCtx, input: &ValidateInput) -> GlobalResult<Option<String>> {
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

	// TODO: Validate build belongs to env/game
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
pub struct DisableTlsPortsInput {
	pub network_ports: util::serde::HashableMap<String, Port>,
}

/// If TLS is not enabled in the cluster, we downgrade all protocols to the non-TLS equivalents.
/// This allows developers to develop locally with the same code they would use in production.
#[activity(DisableTlsPorts)]
pub async fn disable_tls_ports(
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
	actor_id: Uuid,
	env_id: Uuid,
	tags: util::serde::HashableMap<String, String>,
	resources: ActorResources,
	lifecycle: ActorLifecycle,
	image_id: Uuid,
	args: Vec<String>,
	network_mode: NetworkMode,
	environment: util::serde::HashableMap<String, String>,
	network_ports: util::serde::HashableMap<String, Port>,
}

#[activity(InsertDb)]
async fn insert_db(ctx: &ActivityCtx, input: &InsertDbInput) -> GlobalResult<i64> {
	let pool = ctx.sqlite().await?;
	let mut conn = pool.conn().await?;
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

	// TODO: Move this from an op to an activity, and move the sql queries after to their own activity
	// Choose which port to assign for a job's ingress port.
	// This is required because TCP and UDP do not have a `Host` header and thus cannot be re-routed by hostname.
	//
	// If not provided by `ProxiedPort`, then:
	// - HTTP: 80
	// - HTTPS: 443
	// - TCP/TLS: random
	// - UDP: random
	let ingress_ports_res = ctx
		.op(crate::ops::actor::allocate_ingress_ports::Input {
			actor_id: input.actor_id,
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
					INSERT INTO ports_ingress (
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
					INSERT INTO ports_host (
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
struct InsertMetaInput {
	meta: GetMetaOutput,
	root_user_enabled: bool,
}

#[activity(InsertMeta)]
async fn insert_meta(ctx: &ActivityCtx, input: &InsertMetaInput) -> GlobalResult<()> {
	let pool = ctx.sqlite().await?;

	sql_execute!(
		[ctx, pool]
		"
		UPDATE state
		SET
			project_id = ?,
			build_kind = ?,
			build_compression = ?,
			root_user_enabled = ?
		",
		input.meta.project_id,
		input.meta.build_kind as i64,
		input.meta.build_compression as i64,
		input.root_user_enabled,
	)
	.await?;

	Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
struct InsertFdbInput {
	actor_id: Uuid,
	env_id: Uuid,
	tags: util::serde::HashableMap<String, String>,
	create_ts: i64,
}

#[activity(InsertFdb)]
async fn insert_fdb(ctx: &ActivityCtx, input: &InsertFdbInput) -> GlobalResult<()> {
	ctx.fdb()
		.await?
		.run(|tx, _mc| async move {
			let create_ts_key = keys::actor::CreateTsKey::new(input.actor_id);
			tx.set(
				&keys::subspace().pack(&create_ts_key),
				&create_ts_key
					.serialize(input.create_ts)
					.map_err(|x| fdb::FdbBindingError::CustomError(x.into()))?,
			);

			let workflow_id_key = keys::actor::WorkflowIdKey::new(input.actor_id);
			tx.set(
				&keys::subspace().pack(&workflow_id_key),
				&workflow_id_key
					.serialize(ctx.workflow_id())
					.map_err(|x| fdb::FdbBindingError::CustomError(x.into()))?,
			);

			// Add env index key
			let env_actor_key =
				keys::env::ActorKey::new(input.env_id, input.create_ts, input.actor_id);
			let data = keys::env::ActorKeyData {
				is_destroyed: false,
				tags: input.tags.clone().into_iter().collect(),
			};
			tx.set(
				&keys::subspace().pack(&env_actor_key),
				&env_actor_key
					.serialize(data)
					.map_err(|x| fdb::FdbBindingError::CustomError(x.into()))?,
			);

			Ok(())
		})
		.custom_instrument(tracing::info_span!("actor_insert_tx"))
		.await?;

	Ok(())
}

#[derive(Debug, Serialize, Deserialize, Hash)]
pub struct GetMetaInput {
	pub env_id: Uuid,
	pub image_id: Uuid,
}

#[derive(Clone, Debug, Serialize, Deserialize, Hash)]
pub struct GetMetaOutput {
	pub project_id: Uuid,
	pub project_slug: String,
	pub env_slug: String,
	pub build_upload_id: Uuid,
	pub build_file_name: String,
	pub build_kind: BuildKind,
	pub build_compression: BuildCompression,
	pub dc_name_id: String,
	pub dc_display_name: String,
	pub dc_build_delivery_method: BuildDeliveryMethod,
}

#[activity(GetMeta)]
pub async fn get_meta(ctx: &ActivityCtx, input: &GetMetaInput) -> GlobalResult<GetMetaOutput> {
	let dc_id = ctx.config().server()?.rivet.edge()?.datacenter_id;

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

	Ok(GetMetaOutput {
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

pub enum SetupCtx {
	Init {
		network_ports: util::serde::HashableMap<String, Port>,
	},
	Reschedule {
		image_id: Uuid,
	},
}

#[derive(Clone)]
pub struct ActorSetupCtx {
	pub image_id: Uuid,
	pub meta: GetMetaOutput,
	pub resources: protocol::Resources,
	pub artifact_url_stub: String,
	pub fallback_artifact_url: Option<String>,
}

pub async fn setup(
	ctx: &mut WorkflowCtx,
	input: &Input,
	setup: SetupCtx,
) -> GlobalResult<ActorSetupCtx> {
	let image_id = match setup {
		SetupCtx::Init { network_ports } => {
			let tags = input.tags.clone();
			let create_ts = ctx
				.activity(InsertDbInput {
					actor_id: input.actor_id,
					env_id: input.env_id,
					tags: tags.clone(),
					resources: input.resources.clone(),
					lifecycle: input.lifecycle.clone(),
					image_id: input.image_id,
					args: input.args.clone(),
					network_mode: input.network_mode,
					environment: input.environment.clone(),
					network_ports,
				})
				.await?;

			ctx.activity(InsertFdbInput {
				actor_id: input.actor_id,
				env_id: input.env_id,
				tags,
				create_ts,
			})
			.await?;

			input.image_id
		}
		SetupCtx::Reschedule { image_id } => image_id,
	};

	let meta = ctx
		.activity(GetMetaInput {
			env_id: input.env_id,
			image_id,
		})
		.await?;

	ctx.v(2)
		.activity(InsertMetaInput {
			meta: meta.clone(),
			root_user_enabled: input.root_user_enabled,
		})
		.await?;

	let (resources, artifacts_res) = ctx
		.join((
			activity(SelectResourcesInput {
				resources: input.resources.clone(),
			}),
			activity(ResolveArtifactsInput {
				build_upload_id: meta.build_upload_id,
				build_file_name: meta.build_file_name.clone(),
				dc_build_delivery_method: meta.dc_build_delivery_method,
			}),
		))
		.await?;

	Ok(ActorSetupCtx {
		image_id,
		meta,
		resources,
		artifact_url_stub: artifacts_res.artifact_url_stub,
		fallback_artifact_url: artifacts_res.fallback_artifact_url,
	})
}

#[derive(Debug, Serialize, Deserialize, Hash)]
struct SelectResourcesInput {
	resources: ActorResources,
}

#[activity(SelectResources)]
async fn select_resources(
	ctx: &ActivityCtx,
	input: &SelectResourcesInput,
) -> GlobalResult<protocol::Resources> {
	let dc_id = ctx.config().server()?.rivet.edge()?.datacenter_id;

	let tier_res = ctx
		.op(tier::ops::list::Input {
			datacenter_ids: vec![dc_id],
			pegboard: true,
		})
		.await?;
	let tier_dc = unwrap!(tier_res.datacenters.first());
	let mut tiers = tier_dc.tiers.iter().collect::<Vec<_>>();

	// Sort the tiers by cpu
	tiers.sort_by(|a, b| a.cpu.cmp(&b.cpu));

	// Find the first tier that has more CPU and memory than the requested
	// resources
	let tier = unwrap!(
		tiers.iter().find(|t| {
			t.cpu_millicores >= input.resources.cpu_millicores
				&& t.memory >= input.resources.memory_mib
		}),
		"no suitable tier found"
	);

	// runc-compatible resources
	let cpu = tier.rivet_cores_numerator as u64 * 1_000 / tier.rivet_cores_denominator as u64; // Millicore (1/1000 of a core)
	let memory = tier.memory as u64 * (1024 * 1024);
	let memory_max = tier.memory_max as u64 * (1024 * 1024);

	let pool = ctx.sqlite().await?;

	// Write to db
	sql_execute!(
		[ctx, pool]
		"
		UPDATE state
		SET
			selected_resources_cpu_millicores = ?,
			selected_resources_memory_mib = ?
		",
		i64::try_from(cpu)?,
		i64::try_from(tier.memory)?,
	)
	.await?;

	Ok(protocol::Resources {
		cpu,
		memory,
		memory_max,
		disk: tier.disk,
	})
}

#[derive(Debug, Serialize, Deserialize, Hash)]
struct ResolveArtifactsInput {
	build_upload_id: Uuid,
	build_file_name: String,
	dc_build_delivery_method: BuildDeliveryMethod,
}

#[derive(Debug, Serialize, Deserialize, Hash)]
struct ResolveArtifactsOutput {
	artifact_url_stub: String,
	fallback_artifact_url: Option<String>,
}

#[activity(ResolveArtifacts)]
async fn resolve_artifacts(
	ctx: &ActivityCtx,
	input: &ResolveArtifactsInput,
) -> GlobalResult<ResolveArtifactsOutput> {
	let fallback_artifact_url =
		if let BuildDeliveryMethod::S3Direct = input.dc_build_delivery_method {
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
				.key(format!(
					"{upload_id}/{file_name}",
					upload_id = input.build_upload_id,
					file_name = input.build_file_name,
				))
				.presigned(
					s3_util::aws_sdk_s3::presigning::PresigningConfig::builder()
						.expires_in(std::time::Duration::from_secs(15 * 60))
						.build()?,
				)
				.await?;

			let addr_str = presigned_req.uri().to_string();
			tracing::debug!(addr = %addr_str, "resolved artifact s3 presigned request");

			Some(addr_str)
		} else {
			None
		};

	Ok(ResolveArtifactsOutput {
		artifact_url_stub: crate::util::image_artifact_url_stub(
			ctx.config(),
			input.build_upload_id,
			&input.build_file_name,
		)?,
		fallback_artifact_url,
	})
}
