use chirp_worker::prelude::*;
use proto::backend::{
	self,
	pkg::{
		mm::{msg::lobby_find::message::Query, msg::lobby_find_fail::ErrorCode},
		*,
	},
};
use serde_json::json;
use tracing::Instrument;

mod find;
mod limit;
mod verification;

#[derive(Debug, Clone)]
pub struct Player {
	player_id: Uuid,
	token_session_id: Uuid,
	client_info: Option<backend::net::ClientInfo>,
}

/// Pass true for `override_query_state` if executed before the query has been
/// inserted in to Redis.
#[tracing::instrument]
async fn fail(
	ctx: &OperationContext<mm::msg::lobby_find::Message>,
	namespace_id: Uuid,
	query_id: Uuid,
	error_code: ErrorCode,
	force_fail: bool,
) -> GlobalResult<()> {
	tracing::warn!(%namespace_id, ?query_id, ?error_code, ?force_fail, "player create failed");

	let ctx = ctx.base();
	tokio::task::Builder::new()
		.name("mm_lobby_find::fail")
		.spawn(
			async move {
				let res = op!([ctx] mm_lobby_find_fail {
					query_ids: vec![query_id.into()],
					error_code: error_code as i32,
					force_fail: Some(mm::lobby_find_fail::ForceFailContext {
						namespace_id: Some(namespace_id.into()),
					})
				})
				.await;
				match res {
					Ok(_) => {}
					Err(err) => {
						tracing::error!(?err, "failed to call mm_lobby_find_fail");
					}
				}
			}
			.instrument(tracing::info_span!("mm_lobby_try_find_complete")),
		)?;

	Ok(())
}

/// Submits all pending analytic events
#[tracing::instrument]
async fn complete_request(
	client: &chirp_client::Client,
	events: Vec<analytics::msg::event_create::Event>,
) -> GlobalResult<()> {
	msg!([client] analytics::msg::event_create() {
		events: events,
	})
	.await?;
	Ok(())
}

#[worker(name = "mm-lobby-find")]
async fn worker(ctx: &OperationContext<mm::msg::lobby_find::Message>) -> GlobalResult<()> {
	// TODO: Fetch all sessions for the current IP
	// TODO: Map to all players matching the given sessions

	let crdb = ctx.crdb("db-mm-state").await?;
	let mut redis_mm = ctx.redis_mm().await?;

	let mut analytics_events = Vec::new();

	let namespace_id = internal_unwrap!(ctx.namespace_id).as_uuid();
	let query_id = internal_unwrap!(ctx.query_id).as_uuid();
	let join_kind = internal_unwrap_owned!(backend::matchmaker::query::JoinKind::from_i32(
		ctx.join_kind
	));
	let query = internal_unwrap!(ctx.query, "invalid query");
	internal_assert!(!ctx.players.is_empty(), "must have 1 player");

	// Check for stale message
	if ctx.req_dt() > util::duration::seconds(60) {
		tracing::warn!("discarding stale message");
		fail(ctx, namespace_id, query_id, ErrorCode::StaleMessage, true).await?;
		return complete_request(ctx.chirp(), analytics_events).await;
	}

	let ((_namespace, mm_ns_config, dev_team), lobby_group_config) = tokio::try_join!(
		// Namespace and dev team
		fetch_ns_config_and_dev_team(ctx.base(), namespace_id),
		fetch_lobby_group_config(ctx.base(), query),
	)?;

	// Verify dev team status
	if !dev_team.active {
		fail(
			ctx,
			namespace_id,
			query_id,
			ErrorCode::DevTeamInvalidStatus,
			true,
		)
		.await?;
		return complete_request(ctx.chirp(), analytics_events).await;
	}

	// Verify user data
	let verification_success = ctx.bypass_verification
		|| verification::verify(ctx, namespace_id, query_id, query, &lobby_group_config).await?;
	if !verification_success {
		return complete_request(ctx.chirp(), analytics_events).await;
	}

	// Create players
	let players = ctx
		.players
		.iter()
		.map(|player| {
			GlobalResult::Ok(Player {
				player_id: internal_unwrap!(player.player_id).as_uuid(),
				token_session_id: internal_unwrap!(player.token_session_id).as_uuid(),
				client_info: player.client_info.clone(),
			})
		})
		.collect::<GlobalResult<Vec<_>>>()?;

	// Dispatch find create event. We don't use the shared `analytics_events`
	// because those will not be published if the find fails.
	msg!([ctx] analytics::msg::event_create() {
		events: vec![
			analytics::msg::event_create::Event {
				name: "mm.find.create".into(),
				properties_json: Some(serde_json::to_string(&json!({
					"namespace_id": namespace_id,
				}))?),
				..Default::default()
			}
		],
	})
	.await?;

	// Do this as early in the function as possible in order to reduce the
	// resources used by malicious requests
	let validate_perf = ctx.perf().start("validate-player-limit").await;
	if !limit::check_remote_addresses(
		ctx,
		&mut redis_mm,
		&mut analytics_events,
		namespace_id,
		query_id,
		&mm_ns_config,
		&players,
	)
	.await?
	{
		return complete_request(ctx.chirp(), analytics_events).await;
	}
	validate_perf.end();

	// Find the lobby to join
	let auto_create_lobby_id = Uuid::new_v4();
	let find::FindOutput {
		lobby_id,
		region_id,
		lobby_group_id,
	} = if let Some(x) = find::find(
		ctx,
		&crdb,
		&mut redis_mm,
		find::FindOpts {
			namespace_id,
			query_id,
			join_kind,
			players: &players,
			query,
			lobby_group_config: &lobby_group_config,
			auto_create_lobby_id,
		},
	)
	.await?
	{
		x
	} else {
		return complete_request(ctx.chirp(), analytics_events).await;
	};
	let auto_create_lobby = lobby_id == auto_create_lobby_id;

	// Record analytics events
	analytics_events.push(analytics::msg::event_create::Event {
		name: "mm.query.create".into(),
		properties_json: Some(serde_json::to_string(&json!({
			"namespace_id": namespace_id,
			"lobby_id": lobby_id,
			"query_id": query_id,
			"join_kind": join_kind as i32,
			"player_count": players.len(),
			"auto_create_lobby": auto_create_lobby,
		}))?),
		..Default::default()
	});
	for player in &players {
		analytics_events.push(analytics::msg::event_create::Event {
			name: "mm.player.create".into(),
			properties_json: Some(serde_json::to_string(&json!({
				"namespace_id": namespace_id,
				"player_id": player.player_id,
				"lobby_id": lobby_id,
				"query_id": query_id,
				"region_id": region_id,
				"lobby_group_id": lobby_group_id,
				"auto_create_lobby": auto_create_lobby,
				"join_kind": join_kind as i32,
			}))?),
			..Default::default()
		});
	}

	// Insert in to database
	let insert_opts = InsertCrdbOpts {
		namespace_id,
		query_id,
		join_kind,
		players: players.clone(),
		query: query.clone(),
		lobby_id,
		region_id,
		lobby_group_id,
		lobby_group_config: lobby_group_config.clone(),
		auto_create_lobby,
		now_ts: ctx.ts(),
		ray_id: ctx.ray_id(),
	};
	rivet_pools::utils::crdb::tx(&crdb, |tx| {
		Box::pin(insert_to_crdb(tx, insert_opts.clone()))
	})
	.await?;

	// Auto-create lobby if needed.
	//
	// Do this after inserting the players & find results in order to prevent a race condition with
	// removing players if the lobby boot fails.
	let call_try_complete = if auto_create_lobby {
		if let Query::LobbyGroup(backend::matchmaker::query::LobbyGroup {
			auto_create: Some(auto_create),
			..
		}) = query
		{
			let auto_create_perf = ctx.perf().start("auto-create-lobby").await;

			let auto_create_lobby_group_id = internal_unwrap!(auto_create.lobby_group_id).as_uuid();
			let auto_create_region_id = internal_unwrap!(auto_create.region_id).as_uuid();

			tracing::info!(%auto_create_lobby_id, %auto_create_lobby_group_id, %auto_create_region_id, "auto-creating lobby");

			util::inject_latency!();

			let lobby_id = auto_create_lobby_id;
			msg!([ctx] mm::msg::lobby_create(lobby_id) {
				lobby_id: Some(lobby_id.into()),
				namespace_id: Some(namespace_id.into()),
				lobby_group_id: Some(auto_create_lobby_group_id.into()),
				region_id: Some(auto_create_region_id.into()),
				create_ray_id: Some(ctx.ray_id().into()),
				preemptively_created: true,
				creator_user_id: ctx.user_id,
				lobby_config_json: None,
			})
			.await?;

			auto_create_perf.end();
		} else {
			internal_panic!("attempted to auto create lobby for invalid query")
		}

		false
	} else {
		true
	};

	// Publish complete messages
	for player in &players {
		msg!([ctx] mm::msg::player_create_complete(lobby_id, player.player_id) {
			lobby_id: Some(lobby_id.into()),
			player_id: Some(player.player_id.into()),
		})
		.await?;
	}

	// Update idle lobbies
	{
		let ctx = ctx.base();
		tokio::task::Builder::new()
			.name("mm_lobby_find::lobby_idle_update")
			.spawn(
				async move {
					let res = op!([ctx] mm_lobby_idle_update {
						region_id: Some(region_id.into()),
						namespace_id: Some(namespace_id.into()),
					})
					.await;
					match res {
						Ok(_) => {}
						Err(err) => {
							tracing::error!(?err, "failed to call mm_lobby_idle_update");
						}
					}
				}
				.instrument(tracing::info_span!("mm_lobby_idle_update")),
			)?;
	}

	// Try completing the find request
	if call_try_complete {
		let ctx = ctx.base();
		tokio::task::Builder::new()
			.name("mm_lobby_find::find_try_complete")
			.spawn(
				async move {
					let res = op!([ctx] mm_lobby_find_try_complete {
						query_ids: vec![query_id.into()],
					})
					.await;
					match res {
						Ok(_) => {}
						Err(err) => {
							tracing::error!(?err, "failed to call mm_lobby_find_try_complete");
						}
					}
				}
				.instrument(tracing::info_span!("mm_lobby_try_find_complete")),
			)?;
	}

	complete_request(ctx.chirp(), analytics_events).await
}

#[tracing::instrument]
async fn fetch_ns_config_and_dev_team(
	ctx: OperationContext<()>,
	namespace_id: Uuid,
) -> GlobalResult<(
	backend::game::Namespace,
	backend::matchmaker::NamespaceConfig,
	backend::team::DevTeam,
)> {
	let ((ns, dev_team), get_mm_res) = tokio::try_join!(
		{
			let ctx = ctx.base();

			async move {
				let namespace_get_res = op!([ctx] game_namespace_get {
					namespace_ids: vec![namespace_id.into()],
				})
				.await?;
				let namespace = internal_unwrap_owned!(
					namespace_get_res.namespaces.first(),
					"namespace not found"
				);
				let game_id = internal_unwrap_owned!(namespace.game_id);

				// Game
				let game_get_res = op!([ctx] game_get {
					game_ids: vec![game_id],
				})
				.await?;
				let game = internal_unwrap_owned!(game_get_res.games.first());
				let team_id = internal_unwrap_owned!(game.developer_team_id);

				// Dev team
				let team_dev_get_res = op!([ctx] team_dev_get {
					team_ids: vec![team_id],
				})
				.await?;
				let dev_team = internal_unwrap_owned!(team_dev_get_res.teams.first());

				Ok((namespace.clone(), dev_team.clone()))
			}
		},
		op!([ctx] mm_config_namespace_get {
			namespace_ids: vec![namespace_id.into()],
		}),
	)?;

	Ok((
		ns,
		internal_unwrap!(
			internal_unwrap_owned!(get_mm_res.namespaces.first(), "mm namespace not found").config
		)
		.clone(),
		dev_team,
	))
}

#[derive(Debug, Clone)]
pub struct LobbyGroupConfig {
	#[allow(unused)]
	pub version_id: Uuid,
	pub lobby_group: backend::matchmaker::LobbyGroup,
	#[allow(unused)]
	pub lobby_group_meta: backend::matchmaker::LobbyGroupMeta,
	pub lobby_state: Option<backend::matchmaker::Lobby>,
}

/// Fetches the lobby group config (and lobby if direct).
#[tracing::instrument]
async fn fetch_lobby_group_config(
	ctx: OperationContext<()>,
	query: &Query,
) -> GlobalResult<LobbyGroupConfig> {
	// Get lobby group id from query
	let (lobby_group_id, lobby_state) = match query {
		Query::LobbyGroup(backend::matchmaker::query::LobbyGroup { auto_create, .. }) => {
			let auto_create = internal_unwrap!(auto_create);

			(internal_unwrap_owned!(auto_create.lobby_group_id), None)
		}
		Query::Direct(backend::matchmaker::query::Direct { lobby_id, .. }) => {
			let lobby_id = internal_unwrap_owned!(*lobby_id);

			let lobbies_res = op!([ctx] mm_lobby_get {
				lobby_ids: vec![lobby_id],
			})
			.await?;
			let lobby = internal_unwrap_owned!(lobbies_res.lobbies.first(), "lobby not found");

			(
				internal_unwrap_owned!(lobby.lobby_group_id),
				Some(lobby.clone()),
			)
		}
	};
	let lobby_group_id_proto = Some(lobby_group_id);

	// Resolve the version ID
	let resolve_version_res = op!([ctx] mm_config_lobby_group_resolve_version {
		lobby_group_ids: vec![lobby_group_id],
	})
	.await?;
	let version_id = internal_unwrap!(
		internal_unwrap_owned!(
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

	Ok(LobbyGroupConfig {
		version_id,
		lobby_group: (*lobby_group).clone(),
		lobby_group_meta: (*lobby_group_meta).clone(),
		lobby_state,
	})
}

#[derive(Clone)]
struct InsertCrdbOpts {
	namespace_id: Uuid,
	query_id: Uuid,
	join_kind: backend::matchmaker::query::JoinKind,
	players: Vec<Player>,
	query: Query,
	lobby_id: Uuid,
	region_id: Uuid,
	lobby_group_id: Uuid,
	lobby_group_config: LobbyGroupConfig,
	auto_create_lobby: bool,
	now_ts: i64,
	ray_id: Uuid,
}

#[tracing::instrument]
async fn insert_to_crdb(
	tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
	InsertCrdbOpts {
		namespace_id,
		query_id,
		join_kind,
		players,
		query,
		lobby_group_config,
		lobby_id,
		region_id,
		lobby_group_id,
		auto_create_lobby,
		now_ts,
		ray_id,
	}: InsertCrdbOpts,
) -> GlobalResult<()> {
	// Insert preemptive lobby row if needed
	if auto_create_lobby {
		// Insert lobby if needed
		sqlx::query(indoc!(
			"
			INSERT INTO lobbies (
				lobby_id,
				namespace_id,
				region_id,
				lobby_group_id,
				create_ts,
				preemptive_create_ts,
				create_ray_id,
				
				max_players_normal,
				max_players_direct,
				max_players_party,

				is_closed
			)
			VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, false)
			"
		))
		.bind(lobby_id)
		.bind(namespace_id)
		.bind(region_id)
		.bind(lobby_group_id)
		.bind(now_ts)
		.bind(now_ts)
		.bind(ray_id)
		.bind(lobby_group_config.lobby_group.max_players_normal as i64)
		.bind(lobby_group_config.lobby_group.max_players_direct as i64)
		.bind(lobby_group_config.lobby_group.max_players_party as i64)
		.execute(&mut *tx)
		.await?;
	}

	// Insert query
	sqlx::query(indoc!(
		"
		INSERT INTO find_queries (
			query_id,
			namespace_id,
			join_kind,
			lobby_id,
			lobby_auto_created,
			status
		)
		VALUES ($1, $2, $3, $4, $5, $6)
		"
	))
	.bind(query_id)
	.bind(namespace_id)
	.bind(join_kind as i64)
	.bind(lobby_id)
	.bind(auto_create_lobby)
	.bind(util_mm::FindQueryStatus::Pending as i64)
	.execute(&mut *tx)
	.await?;

	// Insert players
	for player in players {
		sqlx::query(indoc!(
			"
			INSERT INTO players (
				player_id,
				lobby_id,
				find_query_id,
				token_session_id,
				remote_address,
				create_ts,
				create_ray_id
			)
			VALUES ($1, $2, $3, $4, $5, $6, $7)
			"
		))
		.bind(player.player_id)
		.bind(lobby_id)
		.bind(query_id)
		.bind(player.token_session_id)
		.bind(
			player
				.client_info
				.as_ref()
				.and_then(|ci| ci.remote_address.clone()),
		)
		.bind(now_ts)
		.bind(ray_id)
		.execute(&mut *tx)
		.await?;
	}

	Ok(())
}
