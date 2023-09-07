use chirp_worker::prelude::*;
use proto::backend::{self, pkg::*};
use redis::AsyncCommands;
use serde_json::json;
use std::ops::Deref;

mod nomad_job;

lazy_static::lazy_static! {
	static ref NOMAD_CONFIG: nomad_client::apis::configuration::Configuration =
		nomad_util::config_from_env().unwrap();

	static ref REDIS_SCRIPT: redis::Script = redis::Script::new(include_str!("../../../redis-scripts/lobby_create.lua"));
}

/// Send a lobby create fail message and cleanup the lobby if needed.
#[tracing::instrument]
async fn fail(
	client: &chirp_client::Client,
	lobby_id: Uuid,
	preemptively_created: bool,
	error_code: mm::msg::lobby_create_fail::ErrorCode,
) -> GlobalResult<()> {
	tracing::warn!(%lobby_id, %preemptively_created, ?error_code, "lobby create failed");

	// Cleanup preemptively inserted lobby.
	//
	// We have to perform a full cleanup instead of just deleting the row since
	// players may have been inserted while waiting for the lobby creation.
	if preemptively_created {
		msg!([client] mm::msg::lobby_cleanup(lobby_id) {
			lobby_id: Some(lobby_id.into()),
		})
		.await?;
	}

	// Send failure message
	msg!([client] mm::msg::lobby_create_fail(lobby_id) {
		lobby_id: Some(lobby_id.into()),
		error_code: error_code as i32,
	})
	.await?;

	Ok(())
}

#[worker(name = "mm-lobby-create")]
async fn worker(ctx: &OperationContext<mm::msg::lobby_create::Message>) -> GlobalResult<()> {
	let crdb = ctx.crdb("db-mm-state").await?;

	let lobby_id = internal_unwrap!(ctx.lobby_id).as_uuid();
	let namespace_id = internal_unwrap!(ctx.namespace_id).as_uuid();
	let lobby_group_id = internal_unwrap!(ctx.lobby_group_id).as_uuid();
	let region_id = internal_unwrap!(ctx.region_id).as_uuid();
	let create_ray_id = ctx.region_id.as_ref().map(common::Uuid::as_uuid);
	let creator_user_id = ctx.creator_user_id.as_ref().map(common::Uuid::as_uuid);

	// Check for stale message
	if ctx.req_dt() > util::duration::seconds(60) {
		tracing::warn!("discarding stale message");
		return fail(
			ctx.chirp(),
			lobby_id,
			ctx.preemptively_created,
			mm::msg::lobby_create_fail::ErrorCode::StaleMessage,
		)
		.await;
	}

	let (namespace, mm_ns_config, (lobby_group, lobby_group_meta, version_id), region, tiers) = tokio::try_join!(
		fetch_namespace(ctx, namespace_id),
		fetch_mm_namespace_config(ctx, namespace_id),
		fetch_lobby_group_config(ctx, lobby_group_id),
		fetch_region(ctx, region_id),
		fetch_tiers(ctx, region_id),
	)?;
	let version = fetch_version(ctx, version_id).await?;

	// Make assertions about the fetched data
	{
		internal_assert_eq!(
			namespace.game_id,
			version.game_id,
			"namespace and version do not belong to the same game"
		);

		// Check if the versions match. If this is not true, then this lobby was
		// likely created while a version was being published. Continue anyway.
		if namespace.version_id != version.version_id {
			tracing::warn!(
				ns_version_id = ?namespace.version_id,
				version_id = ?version.version_id,
				"namespace version is not the same as the given version, likely due to a race condition"
			);
		}
	}

	// Get the relevant lobby group region
	let lobby_group_region = if let Some(x) = lobby_group
		.regions
		.iter()
		.find(|r| r.region_id == ctx.region_id)
	{
		x
	} else {
		return fail(
			ctx.chirp(),
			lobby_id,
			ctx.preemptively_created,
			mm::msg::lobby_create_fail::ErrorCode::RegionNotEnabled,
		)
		.await;
	};

	// Find the relevant tier
	let tier = internal_unwrap_owned!(tiers
		.iter()
		.find(|x| x.tier_name_id == lobby_group_region.tier_name_id));

	let runtime = internal_unwrap!(lobby_group.runtime);
	let runtime = internal_unwrap!(runtime.runtime);
	let runtime_meta = internal_unwrap!(lobby_group_meta.runtime);
	let runtime_meta = internal_unwrap!(runtime_meta.runtime);

	let validate_lobby_count_perf = ctx.perf().start("validate-lobby-count").await;
	if !validate_lobby_count(
		ctx,
		ctx.redis_mm().await?,
		lobby_id,
		&mm_ns_config,
		namespace_id,
	)
	.await?
	{
		return fail(
			ctx.chirp(),
			lobby_id,
			ctx.preemptively_created,
			mm::msg::lobby_create_fail::ErrorCode::LobbyCountOverMax,
		)
		.await;
	}
	validate_lobby_count_perf.end();

	// Create lobby token
	let (lobby_token, token_session_id) = gen_lobby_token(ctx, lobby_id).await?;

	// Insert to database
	let run_id = Uuid::new_v4();
	let insert_opts = UpdateDbOpts {
		lobby_id,
		namespace_id,
		region_id,
		lobby_group_id,
		token_session_id,
		run_id,
		create_ray_id: ctx.ray_id(),
		lobby_group: lobby_group.clone(),
		creator_user_id,
		is_custom: ctx.is_custom,
		publicity: ctx
			.publicity
			.and_then(backend::matchmaker::lobby::Publicity::from_i32),
	};
	rivet_pools::utils::crdb::tx(&crdb, |tx| {
		Box::pin(update_db(ctx.ts(), tx, insert_opts.clone()))
	})
	.await?;

	{
		use util_mm::key;

		let write_perf = ctx.perf().start("write-lobby-redis").await;
		REDIS_SCRIPT
			.arg(ctx.ts())
			.arg(lobby_id.to_string())
			.arg(serde_json::to_string(&key::lobby_config::Config {
				namespace_id,
				region_id,
				lobby_group_id,
				max_players_normal: lobby_group.max_players_normal,
				max_players_party: lobby_group.max_players_party,
				max_players_direct: lobby_group.max_players_direct,
				preemptive: false,
				is_closed: false,
				ready_ts: None,
				is_custom: ctx.is_custom,
				state_json: None,
			})?)
			.arg(ctx.ts() + util_mm::consts::LOBBY_READY_TIMEOUT)
			.key(key::lobby_config(lobby_id))
			.key(key::ns_lobby_ids(namespace_id))
			.key(key::lobby_available_spots(
				namespace_id,
				region_id,
				lobby_group_id,
				util_mm::JoinKind::Normal,
			))
			.key(key::lobby_available_spots(
				namespace_id,
				region_id,
				lobby_group_id,
				util_mm::JoinKind::Party,
			))
			.key(key::lobby_unready())
			.key(key::idle_lobby_ids(namespace_id, region_id, lobby_group_id))
			.key(key::idle_lobby_lobby_group_ids(namespace_id, region_id))
			.key(key::lobby_player_ids(lobby_id))
			.invoke_async(&mut ctx.redis_mm().await?)
			.await?;
		write_perf.end();
	}

	// TODO: Handle this failure case
	// Start the runtime
	match (runtime, runtime_meta) {
		(
			backend::matchmaker::lobby_runtime::Runtime::Docker(runtime),
			backend::matchmaker::lobby_runtime_meta::Runtime::Docker(runtime_meta),
		) => {
			create_docker_job(
				ctx,
				runtime,
				runtime_meta,
				&namespace,
				&version,
				&lobby_group,
				&lobby_group_meta,
				&region,
				tier,
				run_id,
				lobby_id,
				&lobby_token,
			)
			.await?
		}
	};

	msg!([ctx] mm::msg::lobby_create_complete(lobby_id) {
		lobby_id: Some(lobby_id.into()),
		run_id: Some(run_id.into()),
	})
	.await?;

	msg!([ctx] analytics::msg::event_create() {
		events: vec![
			analytics::msg::event_create::Event {
				name: "mm.lobby.create".into(),
				namespace_id: ctx.namespace_id,
				properties_json: Some(serde_json::to_string(&json!({
					"lobby_id": lobby_id,
					"lobby_group_id": lobby_group_id,
					"region_id": region_id,
					"create_ray_id": create_ray_id,
					"preemptively_created": ctx.preemptively_created,
					"tier": tier.tier_name_id,
					"max_players": {
						"normal": lobby_group.max_players_normal,
						"direct": lobby_group.max_players_direct,
						"party": lobby_group.max_players_party,
					},
					"run_id": run_id,
				}))?),
				..Default::default()
			}
		],
	})
	.await?;

	Ok(())
}

#[tracing::instrument]
async fn fetch_region(
	ctx: &OperationContext<mm::msg::lobby_create::Message>,
	region_id: Uuid,
) -> GlobalResult<backend::region::Region> {
	tracing::info!(?region_id, "fetching primary region");
	let primary_get_res = op!([ctx] region_get {
		region_ids: vec![region_id.into()],
	})
	.await?;
	let region = internal_unwrap_owned!(primary_get_res.regions.first(), "region not found");

	Ok(region.clone())
}

#[tracing::instrument]
async fn fetch_tiers(
	ctx: &OperationContext<mm::msg::lobby_create::Message>,
	region_id: Uuid,
) -> GlobalResult<Vec<backend::region::Tier>> {
	let tier_res = op!([ctx] tier_list {
		region_ids: vec![region_id.into()],
	})
	.await?;
	let tier_region = internal_unwrap_owned!(tier_res.regions.first());

	Ok(tier_region.tiers.clone())
}

#[tracing::instrument]
async fn fetch_namespace(
	ctx: &OperationContext<mm::msg::lobby_create::Message>,
	namespace_id: Uuid,
) -> GlobalResult<backend::game::Namespace> {
	let get_res = op!([ctx] game_namespace_get {
		namespace_ids: vec![namespace_id.into()],
	})
	.await?;

	let namespace =
		internal_unwrap_owned!(get_res.namespaces.first(), "namespace not found").clone();

	Ok(namespace)
}

#[tracing::instrument]
async fn fetch_mm_namespace_config(
	ctx: &OperationContext<mm::msg::lobby_create::Message>,
	namespace_id: Uuid,
) -> GlobalResult<backend::matchmaker::NamespaceConfig> {
	let get_res = op!([ctx] mm_config_namespace_get {
		namespace_ids: vec![namespace_id.into()],
	})
	.await?;

	let namespace = internal_unwrap!(
		internal_unwrap_owned!(get_res.namespaces.first(), "namespace not found").config
	)
	.deref()
	.clone();

	Ok(namespace)
}

#[tracing::instrument]
async fn fetch_version(
	ctx: &OperationContext<mm::msg::lobby_create::Message>,
	version_id: Uuid,
) -> GlobalResult<backend::game::Version> {
	let get_res = op!([ctx] game_version_get {
		version_ids: vec![version_id.into()],
	})
	.await?;

	let version = internal_unwrap!(get_res.versions.first(), "version not found")
		.deref()
		.clone();

	Ok(version)
}

#[tracing::instrument]
async fn fetch_lobby_group_config(
	ctx: &OperationContext<mm::msg::lobby_create::Message>,
	lobby_group_id: Uuid,
) -> GlobalResult<(
	backend::matchmaker::LobbyGroup,
	backend::matchmaker::LobbyGroupMeta,
	Uuid,
)> {
	let lobby_group_id_proto = Some(common::Uuid::from(lobby_group_id));

	// Resolve the version ID
	let resolve_version_res = op!([ctx] mm_config_lobby_group_resolve_version {
		lobby_group_ids: vec![lobby_group_id.into()],
	})
	.await?;
	let version_id = internal_unwrap!(
		internal_unwrap!(
			resolve_version_res.versions.first(),
			"lobby group not found"
		)
		.version_id
	)
	.as_uuid();

	// Fetch the config data
	let config_get_res = op!([ctx] mm_config_version_get {
		version_ids: vec![version_id.into()],
	})
	.await?;

	let version = config_get_res.versions.first();
	let version = internal_unwrap!(version, "version config not found");
	let version_config = internal_unwrap!(version.config);
	let version_config_meta = internal_unwrap!(version.config_meta);

	// Find the matching lobby group
	let lobby_group_meta = version_config_meta
		.lobby_groups
		.iter()
		.enumerate()
		.find(|(_, lg)| lg.lobby_group_id == lobby_group_id_proto);
	let (lg_idx, lobby_group_meta) = internal_unwrap!(lobby_group_meta, "lobby group not found");
	let lobby_group = version_config.lobby_groups.get(*lg_idx);
	let lobby_group = internal_unwrap!(lobby_group);

	Ok((
		(*lobby_group).clone(),
		(*lobby_group_meta).clone(),
		version_id,
	))
}

/// Validates that there is room to create one more lobby without going over the lobby count cap.
#[tracing::instrument(skip(redis_mm))]
async fn validate_lobby_count(
	ctx: &OperationContext<mm::msg::lobby_create::Message>,
	mut redis_mm: RedisConn,
	lobby_id: Uuid,
	mm_ns_config: &backend::matchmaker::NamespaceConfig,
	namespace_id: Uuid,
) -> GlobalResult<bool> {
	let lobby_count = redis_mm
		.zcard::<_, u64>(util_mm::key::ns_lobby_ids(namespace_id))
		.await?;
	tracing::info!(?lobby_count, lobby_count_max = ?mm_ns_config.lobby_count_max, "current lobby count");

	Ok(lobby_count < mm_ns_config.lobby_count_max as u64)
}

#[tracing::instrument]
async fn gen_lobby_token(
	ctx: &OperationContext<mm::msg::lobby_create::Message>,
	lobby_id: Uuid,
) -> GlobalResult<(String, Uuid)> {
	let token_res = op!([ctx] token_create {
		issuer: "mm-lobby-create".into(),
		token_config: Some(token::create::request::TokenConfig {
			ttl: util::duration::days(365),
		}),
		refresh_token_config: None,
		client: None,
		kind: Some(token::create::request::Kind::New(token::create::request::KindNew {
			entitlements: vec![
				proto::claims::Entitlement {
					kind: Some(
						proto::claims::entitlement::Kind::MatchmakerLobby(proto::claims::entitlement::MatchmakerLobby {
							lobby_id: Some(lobby_id.into()),
						})
					)
				}
			],
		})),
		label: Some("lobby".into()),
		..Default::default()
	})
	.await?;

	let token = internal_unwrap!(token_res.token);
	let token_session_id = internal_unwrap!(token_res.session_id).as_uuid();

	Ok((token.token.clone(), token_session_id))
}

#[derive(Clone)]
struct UpdateDbOpts {
	lobby_id: Uuid,
	namespace_id: Uuid,
	region_id: Uuid,
	lobby_group_id: Uuid,
	token_session_id: Uuid,
	run_id: Uuid,
	create_ray_id: Uuid,
	lobby_group: backend::matchmaker::LobbyGroup,
	creator_user_id: Option<Uuid>,
	is_custom: bool,
	publicity: Option<backend::matchmaker::lobby::Publicity>,
}

#[tracing::instrument(skip_all)]
async fn update_db(
	now: i64,
	tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
	opts: UpdateDbOpts,
) -> GlobalResult<()> {
	// Check the lobby was created preemptively created and already stopped.
	//
	// This can happen when preemptively created in mm-lobby-find then
	// mm-lobby-cleanup is called.
	//
	// This will lock the lobby for the duration of the transaction
	let lobby_row = sqlx::query_as::<_, (Option<i64>, Option<i64>)>(
		"SELECT stop_ts, preemptive_create_ts FROM lobbies WHERE lobby_id = $1 FOR UPDATE",
	)
	.bind(opts.lobby_id)
	.fetch_optional(&mut **tx)
	.await?;
	if let Some((stop_ts, preemptive_create_ts)) = lobby_row {
		if preemptive_create_ts.is_none() {
			tracing::error!("lobby row exists but is not preemptively created");
			return Ok(());
		}
		if stop_ts.is_some() {
			tracing::info!("lobby already stopped");
			return Ok(());
		}
	}

	// Upsert lobby. May have already been inserted preemptively in
	// mm-lobby-find.
	sqlx::query(indoc!(
		"
		UPSERT INTO lobbies (
			lobby_id,
			namespace_id,
			region_id,
			lobby_group_id,
			token_session_id,
			create_ts,
			run_id,
			create_ray_id,
			
			max_players_normal,
			max_players_direct,
			max_players_party,

			is_closed,
			creator_user_id,
			is_custom,
			publicity
		)
		VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, false, $12, $13, $14)
		"
	))
	.bind(opts.lobby_id)
	.bind(opts.namespace_id)
	.bind(opts.region_id)
	.bind(opts.lobby_group_id)
	.bind(opts.token_session_id)
	.bind(now)
	.bind(opts.run_id)
	.bind(opts.create_ray_id)
	.bind(opts.lobby_group.max_players_normal as i64)
	.bind(opts.lobby_group.max_players_direct as i64)
	.bind(opts.lobby_group.max_players_party as i64)
	.bind(opts.creator_user_id)
	.bind(opts.is_custom)
	.bind(
		opts.publicity
			.unwrap_or(backend::matchmaker::lobby::Publicity::Public) as i32 as i64,
	)
	.execute(&mut **tx)
	.await?;

	Ok(())
}

#[tracing::instrument]
async fn create_docker_job(
	ctx: &OperationContext<mm::msg::lobby_create::Message>,
	runtime: &backend::matchmaker::lobby_runtime::Docker,
	runtime_meta: &backend::matchmaker::lobby_runtime_meta::Docker,
	namespace: &backend::game::Namespace,
	version: &backend::game::Version,
	lobby_group: &backend::matchmaker::LobbyGroup,
	lobby_group_meta: &backend::matchmaker::LobbyGroupMeta,
	region: &backend::region::Region,
	tier: &backend::region::Tier,
	run_id: Uuid,
	lobby_id: Uuid,
	lobby_token: &str,
) -> GlobalResult<()> {
	let namespace_id = internal_unwrap!(namespace.namespace_id).as_uuid();
	let version_id = internal_unwrap!(version.version_id).as_uuid();
	let lobby_group_id = internal_unwrap!(lobby_group_meta.lobby_group_id).as_uuid();
	let region_id = internal_unwrap!(region.region_id).as_uuid();

	let resolve_perf = ctx.perf().start("resolve-image-artifact-url").await;
	let build_id = internal_unwrap!(runtime.build_id).as_uuid();
	let image_artifact_url = resolve_image_artifact_url(ctx, build_id, region).await?;
	resolve_perf.end();

	// Validate build exists and belongs to this game
	let build_id = internal_unwrap!(runtime.build_id).as_uuid();
	let build_get = op!([ctx] build_get {
		build_ids: vec![build_id.into()],
	})
	.await?;
	let build = internal_unwrap_owned!(build_get.builds.first());

	// Generate the Docker job
	let job_spec = nomad_job::gen_lobby_docker_job(
		runtime,
		&build.image_tag,
		tier,
		ctx.lobby_config_json.as_ref(),
	)?;
	let job_spec_json = serde_json::to_string(&job_spec)?;

	// Build proxied ports for each exposed port
	let proxied_ports = runtime
		.ports
		.iter()
		.filter(|port| {
			port.proxy_kind == backend::matchmaker::lobby_runtime::ProxyKind::GameGuard as i32
				&& port.port_range.is_none()
		})
		.map(|port| {
			let job_proxy_protocol = match internal_unwrap_owned!(
				backend::matchmaker::lobby_runtime::ProxyProtocol::from_i32(port.proxy_protocol)
			) {
				backend::matchmaker::lobby_runtime::ProxyProtocol::Http => {
					backend::job::ProxyProtocol::Http as i32
				}
				backend::matchmaker::lobby_runtime::ProxyProtocol::Https => {
					backend::job::ProxyProtocol::Https as i32
				}
				backend::matchmaker::lobby_runtime::ProxyProtocol::Tcp => {
					backend::job::ProxyProtocol::Tcp as i32
				}
				backend::matchmaker::lobby_runtime::ProxyProtocol::TcpTls => {
					backend::job::ProxyProtocol::TcpTls as i32
				}
				backend::matchmaker::lobby_runtime::ProxyProtocol::Udp => {
					backend::job::ProxyProtocol::Udp as i32
				}
			};

			GlobalResult::Ok(job_run::msg::create::ProxiedPort {
				// Match the port label generated in mm-config-version-prepare
				// and in api-matchmaker
				target_nomad_port_label: Some(util_mm::format_nomad_port_label(&port.label)),
				ingress_port: None,
				ingress_hostnames: vec![format!(
					"{}-{}.lobby.{}.{}",
					lobby_id,
					port.label,
					region.name_id,
					util::env::domain_job(),
				)],
				proxy_protocol: job_proxy_protocol,
				ssl_domain_mode: backend::job::SslDomainMode::ParentWildcard as i32,
			})
		})
		.collect::<GlobalResult<Vec<_>>>()?;

	msg!([ctx] job_run::msg::create(run_id) {
		run_id: Some(run_id.into()),
		region_id: Some(region_id.into()),
		parameters: vec![
			job_run::msg::create::Parameter {
				key: "image_artifact_url".into(),
				value: image_artifact_url.to_string(),
			},
			job_run::msg::create::Parameter {
				key: "namespace_id".into(),
				value: namespace_id.to_string(),
			},
			job_run::msg::create::Parameter {
				key: "namespace_name".into(),
				value: namespace.name_id.to_owned(),
			},
			job_run::msg::create::Parameter {
				key: "version_id".into(),
				value: version_id.to_string(),
			},
			job_run::msg::create::Parameter {
				key: "version_name".into(),
				value: version.display_name.to_owned(),
			},
			job_run::msg::create::Parameter {
				key: "lobby_group_id".into(),
				value: lobby_group_id.to_string(),
			},
			job_run::msg::create::Parameter {
				key: "lobby_group_name".into(),
				value: lobby_group.name_id.clone(),
			},
			job_run::msg::create::Parameter {
				key: "lobby_id".into(),
				value: lobby_id.to_string(),
			},
			job_run::msg::create::Parameter {
				key: "lobby_token".into(),
				value: lobby_token.to_owned(),
			},
			job_run::msg::create::Parameter {
				key: "region_id".into(),
				value: region_id.to_string(),
			},
			job_run::msg::create::Parameter {
				key: "region_name".into(),
				value: region.name_id.to_string(),
			},
			job_run::msg::create::Parameter {
				key: "max_players_normal".into(),
				value: lobby_group.max_players_normal.to_string(),
			},
			job_run::msg::create::Parameter {
				key: "max_players_direct".into(),
				value: lobby_group.max_players_direct.to_string(),
			},
			job_run::msg::create::Parameter {
				key: "max_players_party".into(),
				value: lobby_group.max_players_party.to_string(),
			},
		],
		job_spec_json: job_spec_json,
		proxied_ports: proxied_ports,
		..Default::default()
	})
	.await?;

	Ok(())
}

#[tracing::instrument]
async fn resolve_image_artifact_url(
	ctx: &OperationContext<mm::msg::lobby_create::Message>,
	build_id: Uuid,
	region: &backend::region::Region,
) -> GlobalResult<String> {
	let build_res = op!([ctx] build_get {
		build_ids: vec![build_id.into()],
	})
	.await?;
	let build = build_res.builds.first();
	let build = internal_unwrap!(build);
	let upload_id_proto = internal_unwrap!(build.upload_id);

	let upload_res = op!([ctx] upload_get {
		upload_ids: vec![*upload_id_proto],
	})
	.await?;
	let upload = internal_unwrap_owned!(upload_res.uploads.first());

	// Get provider
	let proto_provider = internal_unwrap_owned!(
		backend::upload::Provider::from_i32(upload.provider),
		"invalid upload provider"
	);
	let provider = match proto_provider {
		backend::upload::Provider::Minio => s3_util::Provider::Minio,
		backend::upload::Provider::Backblaze => s3_util::Provider::Backblaze,
		backend::upload::Provider::Aws => s3_util::Provider::Aws,
	};

	match internal_unwrap_owned!(
		std::env::var("RIVET_MM_LOBBY_DELIVERY_METHOD").ok(),
		"missing RIVET_MM_LOBBY_DELIVERY_METHOD"
	)
	.as_str()
	{
		// "s3_direct" => {
		// 	tracing::info!("using s3 direct delivery");

		// 	let bucket = "bucket-build";
		// 	let bucket_screaming = bucket.to_uppercase().replace('-', "_");

		// 	// Build client
		// 	let s3_client = s3_util::Client::from_env_opt(
		// 		bucket,
		// 		provider,
		// 		s3_util::EndpointKind::InternalResolved,
		// 	)
		// 	.await?;

		// 	let upload_id = internal_unwrap!(upload.upload_id).as_uuid();
		// 	let presigned_req = s3_client
		// 		.get_object()
		// 		.bucket(s3_client.bucket())
		// 		.key(format!("{upload_id}/image.tar"))
		// 		.presigned(
		// 			s3_util::aws_sdk_s3::presigning::config::PresigningConfig::builder()
		// 				.expires_in(std::time::Duration::from_secs(15 * 60))
		// 				.build()?,
		// 		)
		// 		.await?;

		// 	let addr = presigned_req.uri().clone();

		// 	let addr_str = addr.to_string();
		// 	tracing::info!(addr = %addr_str, "resolved artifact s3 presigned request");

		// 	Ok(addr_str)
		// }
		"traffic_server" => {
			tracing::info!("using traffic server delivery");

			// HACK: Hardcode ATS IP since this will be replaced shortly
			let ats_url = if region.name_id == "lnd-atl" {
				"10.0.25.2"
			} else if region.name_id == "lnd-fra" {
				"10.0.50.2"
			} else {
				tracing::info!(?region.name_id);
				internal_panic!("invalid region");
			};

			let upload_id = internal_unwrap!(upload.upload_id).as_uuid();
			let addr = format!(
				"http://{ats_url}:9300/s3-cache/{provider}/{namespace}-bucket-build/{upload_id}/image.tar",
				ats_url = ats_url,
				provider = heck::KebabCase::to_kebab_case(provider.to_string().as_str()),
				namespace = util::env::namespace(),
				upload_id = upload_id,
			);

			tracing::info!(%addr, "resolved artifact s3 url");

			Ok(addr)
		}
		_ => {
			internal_panic!("invalid RIVET_MM_LOBBY_DELIVERY_METHOD")
		}
	}
}
