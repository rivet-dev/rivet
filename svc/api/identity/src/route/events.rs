use std::collections::HashMap;

use api_helper::{
	anchor::{WatchIndexQuery, WatchResponse},
	ctx::Ctx,
};
use proto::{
	backend::{self, pkg::*},
	common,
};
use rivet_api::models;
use rivet_convert::{fetch, ApiTryInto};
use rivet_operation::prelude::*;

use crate::{auth::Auth, utils};

// MARK: GET /events/live
pub async fn events(
	ctx: Ctx<Auth>,
	watch_index: WatchIndexQuery,
) -> GlobalResult<models::IdentityWatchEventsResponse> {
	let (current_user_id, game_user) = ctx.auth().dual_user(ctx.op_ctx()).await?;

	let namespace_id = if let Some(game_user) = &game_user {
		Some(unwrap_ref!(game_user.namespace_id).as_uuid())
	} else {
		None
	};

	utils::touch_user_presence(ctx.op_ctx().base(), current_user_id, false);

	// Wait for an update if needed
	let EventsWaitResponse {
		new_mm_lobby_joins,
		removed_team_ids: _,
		user_update_ts,
		last_update_ts,
		valid_anchor: _,
	} = if let Some(anchor) = &watch_index.to_consumer()? {
		events_wait(&ctx, anchor, current_user_id).await?
	} else {
		EventsWaitResponse {
			new_mm_lobby_joins: Vec::new(),
			removed_team_ids: Vec::new(),

			user_update_ts: None,
			last_update_ts: None,
			valid_anchor: false,
		}
	};
	let last_update_ts = last_update_ts.unwrap_or_else(util::timestamp::now);

	// Process events
	let (user_update_events, mm_lobby_events) = tokio::try_join!(
		async {
			if let Some(ts) = user_update_ts {
				Ok(Some(
					process_user_update_events(&ctx, ts, current_user_id, game_user).await?,
				))
			} else {
				Ok(None)
			}
		},
		process_mm_lobby_events(&ctx, namespace_id, new_mm_lobby_joins),
	)?;

	// Merge and sort events
	let mut events = user_update_events
		.into_iter()
		.chain(mm_lobby_events)
		.collect::<Vec<_>>();
	// TODO: Shouldn't be sorting strings
	events.sort_by(|x, y| x.ts.cmp(&y.ts));

	Ok(models::IdentityWatchEventsResponse {
		events,
		watch: WatchResponse::new_as_model(last_update_ts),
	})
}

struct EventsWaitResponse {
	new_mm_lobby_joins: Vec<(i64, backend::user::event::MatchmakerLobbyJoin)>,

	removed_team_ids: Vec<(i64, Uuid)>,

	/// Timestamp of the last message that was received from the tail. This is
	/// used to build the new watch index.
	last_update_ts: Option<i64>,

	/// Timestamp of the last user update.
	user_update_ts: Option<i64>,

	/// If the provided anchor was valid. If false, the tail will return
	/// immediately and we'll do a fresh pull of all events.
	valid_anchor: bool,
}

// MARK: Wait
async fn events_wait(
	ctx: &Ctx<Auth>,
	anchor: &chirp_client::TailAnchor,
	current_user_id: Uuid,
) -> GlobalResult<EventsWaitResponse> {
	// TODO: Watch for changes in direct chats, teams, and parties. i.e. profile
	// changes, activity changes, party updates, etc. This can be done by a
	// worker that publishes user-specific updates for all present users.

	let thread_tail =
		tail_all!([ctx, anchor, chirp_client::TailAllConfig::wait_return_immediately()] user::msg::event(current_user_id))
			.await?;

	let last_update_ts = thread_tail.messages.last().map(|msg| msg.msg_ts());

	tracing::info!(?thread_tail, "thread tail");

	// Decode messages
	let mut new_mm_lobby_joins = Vec::new();
	let mut removed_team_ids = Vec::new();
	let mut user_update_ts = None;
	for msg in thread_tail.messages {
		let event = unwrap_ref!(msg.event);

		if let Some(event) = &event.kind {
			match event {
				backend::user::event::event::Kind::MatchmakerLobbyJoin(mm_lobby_join) => {
					new_mm_lobby_joins.push((msg.msg_ts(), mm_lobby_join.clone()));
				}
				backend::user::event::event::Kind::UserUpdate(_) => {
					user_update_ts = Some(msg.msg_ts());
				}
				backend::user::event::event::Kind::PresenceUpdate(_) => {
					user_update_ts = Some(msg.msg_ts());
				}
				backend::user::event::event::Kind::TeamMemberRemove(team) => {
					removed_team_ids.push((msg.msg_ts(), unwrap_ref!(team.team_id).as_uuid()));
				}
			}
		} else {
			tracing::warn!(?event, "unknown user event kind");
		}
	}

	Ok(EventsWaitResponse {
		new_mm_lobby_joins,
		removed_team_ids,
		user_update_ts,
		last_update_ts,
		valid_anchor: thread_tail.anchor_status != chirp_client::TailAllAnchorStatus::Expired,
	})
}

// MARK: User update
async fn process_user_update_events(
	ctx: &Ctx<Auth>,
	ts: i64,
	current_user_id: Uuid,
	game_user: Option<game_user::get::response::GameUser>,
) -> GlobalResult<models::IdentityGlobalEvent> {
	let identities = fetch::identity::profiles(
		ctx.op_ctx(),
		current_user_id,
		game_user.and_then(|x| x.game_user_id.map(|id| *id)),
		vec![current_user_id],
	)
	.await?;
	let profile = unwrap_with!(identities.into_iter().next(), IDENTITY_NOT_FOUND);

	Ok(models::IdentityGlobalEvent {
		ts: util::timestamp::to_string(ts)?,
		kind: Box::new(models::IdentityGlobalEventKind {
			identity_update: Some(Box::new(models::IdentityGlobalEventIdentityUpdate {
				identity: Box::new(profile),
			})),
			..Default::default()
		}),
		notification: None,
	})
}

// MARK: Matchmaker lobby join
async fn process_mm_lobby_events(
	ctx: &Ctx<Auth>,
	namespace_id: Option<Uuid>,
	mm_lobby_joins: Vec<(i64, backend::user::event::MatchmakerLobbyJoin)>,
) -> GlobalResult<Vec<models::IdentityGlobalEvent>> {
	// Ignore these events if there's no namespace ID
	let namespace_id = if let Some(namespace_id) = namespace_id {
		namespace_id
	} else {
		return Ok(Vec::new());
	};
	let namespace_id_proto = common::Uuid::from(namespace_id);

	// Ignore if no mm_lobby_joins provided
	if mm_lobby_joins.is_empty() {
		return Ok(Vec::new());
	}

	// Fetch all lobbies
	let lobby_res = op!([ctx] mm_lobby_get {
		lobby_ids: mm_lobby_joins
			.iter()
			.filter_map(|(_, x)| x.lobby_id)
			.collect(),
		include_stopped: true,
	})
	.await?;

	let mut events = Vec::new();

	// Add join event.
	//
	// Only use the most recent event since we don't care about
	// multiple joins.
	//
	// Filter by namespace to only include events relevant to the current game
	// user.
	let most_recent_join = mm_lobby_joins
		.iter()
		.filter(|(_, x)| {
			x.namespace_id
				.as_ref()
				.map_or(false, |x| *x == namespace_id_proto)
		})
		.last();
	if let Some((ts, join)) = most_recent_join {
		if let Some(lobby_proto) = lobby_res
			.lobbies
			.iter()
			.find(|&x| x.lobby_id == join.lobby_id)
		{
			let lobby = fetch_lobby(ctx.op_ctx(), lobby_proto, &join.player_token).await?;

			events.push(models::IdentityGlobalEvent {
				ts: util::timestamp::to_string(*ts)?,
				kind: Box::new(models::IdentityGlobalEventKind {
					matchmaker_lobby_join: Some(Box::new(lobby)),
					..Default::default()
				}),
				notification: None,
			});
		} else {
			tracing::warn!(?join, "could not find lobby for join event");
		};
	}

	Ok(events)
}

async fn fetch_lobby(
	ctx: &OperationContext<()>,
	lobby: &backend::matchmaker::Lobby,
	player_token: &str,
) -> GlobalResult<models::IdentityGlobalEventMatchmakerLobbyJoin> {
	let lobby_id = unwrap_ref!(lobby.lobby_id).as_uuid();
	let region_id = unwrap_ref!(lobby.region_id);
	let lobby_group_id = unwrap_ref!(lobby.lobby_group_id);
	let run_id = unwrap_ref!(lobby.run_id);

	// Fetch lobby run data
	let (run_res, version) = tokio::try_join!(
		// Fetch the job run
		async {
			op!([ctx] job_run_get {
				run_ids: vec![*run_id],
			})
			.await
			.map_err(Into::<GlobalError>::into)
		},
		// Fetch the version
		async {
			let version_res = op!([ctx] mm_config_lobby_group_resolve_version {
				lobby_group_ids: vec![*lobby_group_id],
			})
			.await?;

			let version_id = unwrap!(version_res.versions.first());
			let version_id = unwrap_ref!(version_id.version_id);
			let version_res = op!([ctx] mm_config_version_get {
				version_ids: vec![*version_id],
			})
			.await?;
			let version = unwrap!(version_res.versions.first());

			GlobalResult::Ok(version.clone())
		}
	)?;

	// Match the version
	let version_config = unwrap_ref!(version.config);
	let version_meta = unwrap_ref!(version.config_meta);
	let (lobby_group_config, _lobby_group_meta) = unwrap!(version_config
		.lobby_groups
		.iter()
		.zip(version_meta.lobby_groups.iter())
		.find(|(_, meta)| meta.lobby_group_id.as_ref() == Some(lobby_group_id)));
	let lobby_runtime = unwrap_ref!(lobby_group_config.runtime);
	#[allow(clippy::infallible_destructuring_match)]
	let docker_runtime = match unwrap_ref!(lobby_runtime.runtime) {
		backend::matchmaker::lobby_runtime::Runtime::Docker(x) => x,
	};

	// Convert the ports to client-friendly ports
	let run = unwrap!(run_res.runs.first());

	let ports = docker_runtime
		.ports
		.iter()
		.map(|port| build_port(run, port))
		.filter_map(|x| x.transpose())
		.collect::<GlobalResult<HashMap<_, _>>>()?;

	// Fetch region data
	let region_res = op!([ctx] region_get {
		region_ids: vec![*region_id],
	})
	.await?;
	let region_proto = unwrap!(region_res.regions.first());
	let region = models::MatchmakerJoinRegion {
		region_id: region_proto.name_id.clone(),
		display_name: region_proto.region_display_name.clone(),
	};

	// TODO: Gracefully catch errors from this

	Ok(models::IdentityGlobalEventMatchmakerLobbyJoin {
		lobby: Box::new(models::MatchmakerJoinLobby {
			lobby_id,
			region: Box::new(region),
			..Default::default()
		}),
		ports,
		player: Box::new(models::MatchmakerJoinPlayer {
			token: player_token.to_string(),
		}),
	})
}

// TODO: Copied from api-matchmaker
fn build_port(
	run: &backend::job::Run,
	port: &backend::matchmaker::lobby_runtime::Port,
) -> GlobalResult<Option<(String, models::MatchmakerJoinPort)>> {
	use backend::{
		job::ProxyProtocol as JobProxyProtocol,
		matchmaker::lobby_runtime::{ProxyKind as MmProxyKind, ProxyProtocol as MmProxyProtocol},
	};

	let proxy_kind = unwrap!(MmProxyKind::from_i32(port.proxy_kind));
	let mm_proxy_protocol = unwrap!(MmProxyProtocol::from_i32(port.proxy_protocol));

	let join_info_port = match (proxy_kind, mm_proxy_protocol) {
		(
			MmProxyKind::GameGuard,
			MmProxyProtocol::Http
			| MmProxyProtocol::Https
			| MmProxyProtocol::Tcp
			| MmProxyProtocol::TcpTls
			| MmProxyProtocol::Udp,
		) => {
			run.proxied_ports
				.iter()
				// Decode the proxy protocol
				.filter_map(|proxied_port| {
					match JobProxyProtocol::from_i32(proxied_port.proxy_protocol) {
						Some(x) => Some((proxied_port, x)),
						None => {
							tracing::error!(?proxied_port, "could not decode job proxy protocol");
							None
						}
					}
				})
				// Match the matchmaker port with the job port that matches the same
				// port and protocol
				.filter(|(proxied_port, job_proxy_protocol)| {
					test_mm_and_job_proxy_protocol_eq(mm_proxy_protocol, *job_proxy_protocol)
						&& proxied_port.target_nomad_port_label
							== Some(util_mm::format_nomad_port_label(&port.label))
				})
				// Extract the port's host. This should never be `None`.
				.filter_map(|(proxied_port, _)| {
					proxied_port
						.ingress_hostnames
						.first()
						.map(|hostname| (proxied_port, hostname))
				})
				.map(|(proxied_port, hostname)| models::MatchmakerJoinPort {
					host: Some(format!("{}:{}", hostname, proxied_port.ingress_port)),
					hostname: hostname.clone(),
					port: Some(proxied_port.ingress_port as i32),
					port_range: None,
					is_tls: matches!(
						mm_proxy_protocol,
						MmProxyProtocol::Https | MmProxyProtocol::TcpTls
					),
				})
				.next()
		}
		(MmProxyKind::None, MmProxyProtocol::Tcp | MmProxyProtocol::Udp) => {
			let port_range = unwrap_ref!(port.port_range);

			let run_meta = unwrap_ref!(run.run_meta);
			let Some(backend::job::run_meta::Kind::Nomad(run_meta_nomad)) = &run_meta.kind else {
				bail!("invalid nomad run meta kind")
			};
			let node_public_ipv4 = unwrap_ref!(run_meta_nomad.node_public_ipv4);

			Some(models::MatchmakerJoinPort {
				host: None,
				hostname: node_public_ipv4.clone(),
				port: None,
				port_range: Some(Box::new(models::MatchmakerJoinPortRange {
					min: port_range.min.api_try_into()?,
					max: port_range.max.api_try_into()?,
				})),
				is_tls: false,
			})
		}
		(
			MmProxyKind::None,
			MmProxyProtocol::Http | MmProxyProtocol::Https | MmProxyProtocol::TcpTls,
		) => {
			bail!("invalid http proxy protocol with host network")
		}
	};

	GlobalResult::Ok(join_info_port.map(|x| (port.label.clone(), x)))
}

fn test_mm_and_job_proxy_protocol_eq(
	mm_proxy_protocol: backend::matchmaker::lobby_runtime::ProxyProtocol,
	job_proxy_protocol: backend::job::ProxyProtocol,
) -> bool {
	use backend::{job::ProxyProtocol as JPP, matchmaker::lobby_runtime::ProxyProtocol as MPP};

	matches!(
		(mm_proxy_protocol, job_proxy_protocol),
		(MPP::Http, JPP::Http)
			| (MPP::Https, JPP::Https)
			| (MPP::Tcp, JPP::Tcp)
			| (MPP::TcpTls, JPP::TcpTls)
			| (MPP::Udp, JPP::Udp)
	)
}
