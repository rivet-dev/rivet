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

	let crdb = ctx.crdb().await?;
	let mut redis_mm = ctx.redis_mm().await?;

	let mut analytics_events = Vec::new();

	let namespace_id = unwrap_ref!(ctx.namespace_id).as_uuid();
	let query_id = unwrap_ref!(ctx.query_id).as_uuid();
	let join_kind = unwrap!(backend::matchmaker::query::JoinKind::from_i32(
		ctx.join_kind
	));
	let query = unwrap_ref!(ctx.query, "invalid query");
	ensure!(!ctx.players.is_empty(), "must have 1 player");

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

	// Create players
	let players = ctx
		.players
		.iter()
		.map(|player| {
			GlobalResult::Ok(Player {
				player_id: unwrap_ref!(player.player_id).as_uuid(),
				token_session_id: unwrap_ref!(player.token_session_id).as_uuid(),
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

	// Verify user data
	if !ctx.bypass_verification {
		let verification_res = util_mm::verification::verify_config(
			&ctx.base(),
			&util_mm::verification::VerifyConfigOpts {
				kind: match query {
					Query::LobbyGroup(_) => util_mm::verification::ConnectionKind::Find,
					Query::Direct(_) => util_mm::verification::ConnectionKind::Join,
				},
				namespace_id,
				user_id: ctx.user_id.map(|id| id.as_uuid()),
				lobby_groups: &lobby_group_config.lobby_groups,
				lobby_group_meta: &lobby_group_config.lobby_group_meta,
				lobby_info: lobby_group_config.lobby_info.as_ref(),
				lobby_state_json: lobby_group_config.lobby_state_json.as_deref(),
				verification_data_json: ctx.verification_data_json.as_deref(),
				lobby_config_json: None,
				custom_lobby_publicity: None,
			},
		)
		.await;
		if let Err(err) = verification_res {
			// Reduces verbosity
			let err_branch = |err_code| async move {
				fail(ctx, namespace_id, query_id, err_code, true).await?;
				complete_request(ctx.chirp(), analytics_events).await
			};

			let res = if err.is(formatted_error::code::MATCHMAKER_FIND_DISABLED) {
				err_branch(ErrorCode::FindDisabled).await
			} else if err.is(formatted_error::code::MATCHMAKER_JOIN_DISABLED) {
				err_branch(ErrorCode::JoinDisabled).await
			} else if err.is(formatted_error::code::MATCHMAKER_REGISTRATION_REQUIRED) {
				err_branch(ErrorCode::RegistrationRequired).await
			} else if err.is(formatted_error::code::MATCHMAKER_IDENTITY_REQUIRED) {
				err_branch(ErrorCode::IdentityRequired).await
			} else if err.is(formatted_error::code::MATCHMAKER_VERIFICATION_FAILED) {
				err_branch(ErrorCode::VerificationFailed).await
			} else if err.is(formatted_error::code::MATCHMAKER_VERIFICATION_REQUEST_FAILED) {
				err_branch(ErrorCode::VerificationRequestFailed).await
			} else {
				Err(err)
			};

			return res;
		}
	}

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
		let ctx = ctx.clone();
		Box::pin(insert_to_crdb(ctx, tx, insert_opts.clone()))
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

			let auto_create_lobby_group_id = unwrap_ref!(auto_create.lobby_group_id).as_uuid();
			let auto_create_region_id = unwrap_ref!(auto_create.region_id).as_uuid();

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
				is_custom: false,
				publicity: None,
				lobby_config_json: None,
			})
			.await?;

			auto_create_perf.end();
		} else {
			bail!("attempted to auto create lobby for invalid query")
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
				let namespace =
					unwrap!(namespace_get_res.namespaces.first(), "namespace not found");
				let game_id = unwrap!(namespace.game_id);

				// Game
				let game_get_res = op!([ctx] game_get {
					game_ids: vec![game_id],
				})
				.await?;
				let game = unwrap!(game_get_res.games.first());
				let team_id = unwrap!(game.developer_team_id);

				// Dev team
				let team_dev_get_res = op!([ctx] team_dev_get {
					team_ids: vec![team_id],
				})
				.await?;
				let dev_team = unwrap!(team_dev_get_res.teams.first());

				Ok((namespace.clone(), dev_team.clone()))
			}
		},
		op!([ctx] mm_config_namespace_get {
			namespace_ids: vec![namespace_id.into()],
		}),
	)?;

	let mm_config =
		unwrap_ref!(unwrap!(get_mm_res.namespaces.first(), "mm namespace not found").config);

	Ok((ns, mm_config.clone(), dev_team))
}

#[derive(Debug, Clone)]
pub struct LobbyGroupConfig {
	#[allow(unused)]
	pub version_id: Uuid,
	pub lobby_groups: Vec<backend::matchmaker::LobbyGroup>,
	pub lobby_group_meta: Vec<backend::matchmaker::LobbyGroupMeta>,
	pub lobby_info: Option<backend::matchmaker::Lobby>,
	pub lobby_state_json: Option<String>,
}

/// Fetches the lobby group config (and lobby if direct).
#[tracing::instrument]
async fn fetch_lobby_group_config(
	ctx: OperationContext<()>,
	query: &Query,
) -> GlobalResult<LobbyGroupConfig> {
	// Get lobby group id from query
	let (lobby_group_ids, lobby_info, lobby_state_json) = match query {
		Query::LobbyGroup(backend::matchmaker::query::LobbyGroup {
			lobby_group_ids,
			auto_create,
			..
		}) => {
			let lobby_group_ids = if let Some(auto_create) = auto_create {
				vec![unwrap!(auto_create.lobby_group_id)]
			} else {
				lobby_group_ids.clone()
			};

			(lobby_group_ids, None, None)
		}
		Query::Direct(backend::matchmaker::query::Direct { lobby_id, .. }) => {
			let lobby_id = unwrap!(*lobby_id);

			let (lobbies_res, lobby_states_res) = tokio::try_join!(
				op!([ctx] mm_lobby_get {
					lobby_ids: vec![lobby_id],
					include_stopped: true,
				}),
				op!([ctx] mm_lobby_state_get {
					lobby_ids: vec![lobby_id],
				}),
			)?;
			let lobby = unwrap!(lobbies_res.lobbies.into_iter().next(), "lobby not found");
			let lobby_group_id = unwrap!(lobby.lobby_group_id);
			let lobby_state = unwrap!(lobby_states_res.lobbies.into_iter().next());

			(vec![lobby_group_id], Some(lobby), lobby_state.state_json)
		}
	};

	// Resolve the version ID (should all be from the same version)
	let resolve_version_res = op!([ctx] mm_config_lobby_group_resolve_version {
		lobby_group_ids: lobby_group_ids.clone(),
	})
	.await?;
	let version_id = unwrap_ref!(
		unwrap!(
			resolve_version_res.versions.first(),
			"lobby group version not found"
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
	let version = unwrap_ref!(version, "version config not found");
	let version_config = unwrap_ref!(version.config);
	let version_config_meta = unwrap_ref!(version.config_meta);

	// Find the matching lobby groups
	let (lobby_groups, lobby_group_meta) = lobby_group_ids
		.iter()
		.map(|id| {
			let lobby_group_meta = version_config_meta
				.lobby_groups
				.iter()
				.enumerate()
				.find(|(_, lg)| lg.lobby_group_id == Some(*id));
			let (lg_idx, lobby_group_meta) = unwrap!(lobby_group_meta, "lobby group not found");
			let lobby_group = version_config.lobby_groups.get(lg_idx);
			let lobby_group = unwrap!(lobby_group);

			Ok((lobby_group.clone(), lobby_group_meta.clone()))
		})
		.collect::<GlobalResult<Vec<_>>>()?
		.into_iter()
		.unzip::<_, _, Vec<_>, Vec<_>>();

	Ok(LobbyGroupConfig {
		version_id,
		lobby_groups,
		lobby_group_meta,
		lobby_info,
		lobby_state_json,
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
	ctx: OperationContext<mm::msg::lobby_find::Message>,
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
		// Lobby group will exist if the lobby was auto created
		let lobby_group = unwrap!(lobby_group_config.lobby_groups.first());

		// Insert lobby if needed
		sql_query!(
			[ctx, &mut **tx]
			"
			INSERT INTO db_mm_state.lobbies (
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
			",
			lobby_id,
			namespace_id,
			region_id,
			lobby_group_id,
			now_ts,
			now_ts,
			ray_id,
			lobby_group.max_players_normal as i64,
			lobby_group.max_players_direct as i64,
			lobby_group.max_players_party as i64,
		)
		.await?;
	}

	// Insert query
	sql_query!(
		[ctx, &mut **tx]
		"
		INSERT INTO db_mm_state.find_queries (
			query_id,
			namespace_id,
			join_kind,
			lobby_id,
			lobby_auto_created,
			status
		)
		VALUES ($1, $2, $3, $4, $5, $6)
		",
		query_id,
		namespace_id,
		join_kind as i64,
		lobby_id,
		auto_create_lobby,
		util_mm::FindQueryStatus::Pending as i64,
	)
	.await?;

	// Insert players
	for player in players {
		sql_query!(
			[ctx, &mut **tx]
			"
			INSERT INTO db_mm_state.players (
				player_id,
				lobby_id,
				find_query_id,
				token_session_id,
				remote_address,
				create_ts,
				create_ray_id
			)
			VALUES ($1, $2, $3, $4, $5, $6, $7)
			",
			player.player_id,
			lobby_id,
			query_id,
			player.token_session_id,
			player
				.client_info
				.as_ref()
				.and_then(|ci| ci.remote_address.clone()),
			now_ts,
			ray_id,
		)
		.await?;
	}

	Ok(())
}
