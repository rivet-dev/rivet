use std::{
	collections::HashMap,
	convert::TryInto,
	hash::{DefaultHasher, Hasher},
	net::IpAddr,
	time::Duration,
};

use chirp_workflow::prelude::*;
use cluster::types::BuildDeliveryMethod;
use futures_util::FutureExt;
use rand::Rng;
use rivet_operation::prelude::proto::backend;

use crate::types::{
	build::{BuildCompression, BuildKind},
	GameGuardProtocol, GameRuntime, NetworkMode, Routing, ServerResources,
};

pub mod nomad;
pub mod pegboard;

// In ms, a small amount of time to separate the completion of the drain to the deletion of the
// cluster server. We want the drain to complete first.
const DRAIN_PADDING_MS: i64 = 10000;

// TODO: Restructure traefik to get rid of this
const TRAEFIK_GRACE_PERIOD: Duration = Duration::from_secs(2);

#[derive(Default, Clone)]
pub(crate) struct GameGuardUnnest {
	pub port_names: Vec<String>,
	pub port_numbers: Vec<Option<i32>>,
	pub gg_ports: Vec<Option<i32>>,
	pub protocols: Vec<i32>,
}

#[derive(Default, Clone)]
pub(crate) struct HostUnnest {
	pub port_names: Vec<String>,
	pub port_numbers: Vec<Option<i32>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Input {
	pub server_id: Uuid,
	pub env_id: Uuid,
	pub datacenter_id: Uuid,
	pub cluster_id: Uuid,
	pub runtime: GameRuntime,
	pub tags: HashMap<String, String>,
	pub resources: ServerResources,
	pub kill_timeout_ms: i64,
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
	pub internal_port: Option<i32>,
	pub routing: Routing,
}

#[workflow]
pub async fn ds_server(ctx: &mut WorkflowCtx, input: &Input) -> GlobalResult<()> {
	match input.runtime {
		GameRuntime::Nomad => {
			ctx.workflow(nomad::Input {
				server_id: input.server_id,
				env_id: input.env_id,
				datacenter_id: input.datacenter_id,
				cluster_id: input.cluster_id,
				tags: input.tags.clone(),
				resources: input.resources.clone(),
				kill_timeout_ms: input.kill_timeout_ms,
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
		GameRuntime::Pegboard => {
			ctx.workflow(pegboard::Input {
				server_id: input.server_id,
				env_id: input.env_id,
				datacenter_id: input.datacenter_id,
				cluster_id: input.cluster_id,
				tags: input.tags.clone(),
				resources: input.resources.clone(),
				kill_timeout_ms: input.kill_timeout_ms,
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
pub(crate) struct InsertDbInput {
	server_id: Uuid,
	env_id: Uuid,
	datacenter_id: Uuid,
	cluster_id: Uuid,
	tags: util::serde::HashableMap<String, String>,
	resources: ServerResources,
	kill_timeout_ms: i64,
	image_id: Uuid,
	args: Vec<String>,
	network_mode: NetworkMode,
	environment: util::serde::HashableMap<String, String>,
	network_ports: util::serde::HashableMap<String, Port>,
}

#[activity(InsertDb)]
pub(crate) async fn insert_db(ctx: &ActivityCtx, input: &InsertDbInput) -> GlobalResult<()> {
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
				input.resources.cpu_millicores as i32,
				input.resources.memory_mib as i32,
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
pub(crate) struct GetBuildAndDcInput {
	datacenter_id: Uuid,
	image_id: Uuid,
}

#[derive(Debug, Serialize, Deserialize, Hash)]
pub(crate) struct GetBuildAndDcOutput {
	build_upload_id: Uuid,
	build_file_name: String,
	build_kind: BuildKind,
	build_compression: BuildCompression,
	dc_name_id: String,
	dc_build_delivery_method: BuildDeliveryMethod,
}

#[activity(GetBuildAndDc)]
pub(crate) async fn get_build_and_dc(
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

#[message("ds_server_create_complete")]
pub struct CreateComplete {}

#[message("ds_server_create_failed")]
pub struct CreateFailed {}

#[signal("ds_server_destroy")]
pub struct Destroy {
	pub override_kill_timeout_ms: Option<i64>,
}

#[message("ds_server_destroy_started")]
pub struct DestroyStarted {}

#[message("ds_server_destroy_complete")]
pub struct DestroyComplete {}

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
pub(crate) async fn resolve_image_artifact_url(
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
			tracing::info!("using s3 direct delivery");

			// Build client
			let s3_client =
				s3_util::Client::from_env_opt("bucket-build", s3_util::EndpointKind::External)
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
				namespace = util::env::namespace(),
				upload_id = upload_id,
			);

			tracing::info!(%addr, "resolved artifact s3 url");

			Ok(addr)
		}
	}
}

/// Choose which port to assign for a job's ingress port.
///
/// If not provided by `ProxiedPort`, then:
/// - HTTP: 80
/// - HTTPS: 443
/// - TCP/TLS: random
/// - UDP: random
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

/// This is very poorly written for TCP & UDP ports and may bite us in the ass
/// some day. See https://linear.app/rivet-gg/issue/RVT-1799
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
				FROM db_ds.servers AS s
				JOIN db_ds.docker_ports_protocol_game_guard AS p
				ON s.server_id = p.server_id
				WHERE
					s.destroy_ts IS NULL AND
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
