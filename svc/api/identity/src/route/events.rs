use api_helper::{
	anchor::{WatchIndexQuery, WatchResponse},
	ctx::Ctx,
};
use proto::{
	backend::{self, pkg::*},
	common,
};
use rivet_api::models;
use rivet_convert::{convert, fetch, ApiTryInto};
use rivet_operation::prelude::*;
use std::collections::{HashMap, HashSet};

use crate::{auth::Auth, utils};

const CHAT_THREAD_HISTORY: usize = 64;

// MARK: GET /events/live
pub async fn events(
	ctx: Ctx<Auth>,
	watch_index: WatchIndexQuery,
) -> GlobalResult<models::IdentityWatchEventsResponse> {
	let (current_user_id, game_user) = ctx.auth().dual_user(ctx.op_ctx()).await?;

	let namespace_id = if let Some(game_user) = &game_user {
		Some(internal_unwrap!(game_user.namespace_id).as_uuid())
	} else {
		None
	};

	utils::touch_user_presence(ctx.op_ctx().base(), current_user_id, false);

	// Wait for an update if needed
	let EventsWaitResponse {
		new_messages,
		new_chat_reads,
		new_mm_lobby_joins,
		removed_team_ids,
		user_update_ts,
		last_update_ts,
		valid_anchor,
	} = if let Some(anchor) = &watch_index.to_consumer()? {
		events_wait(&ctx, anchor, current_user_id).await?
	} else {
		EventsWaitResponse {
			new_messages: Vec::new(),
			new_chat_reads: Vec::new(),
			new_mm_lobby_joins: Vec::new(),
			removed_team_ids: Vec::new(),

			user_update_ts: None,
			last_update_ts: None,
			valid_anchor: false,
		}
	};
	let last_update_ts = last_update_ts.unwrap_or_else(util::timestamp::now);

	// Process events
	let (msg_events, chat_read_events, user_update_events, mm_lobby_events, thread_remove_events) =
		tokio::try_join!(
			process_msg_events(&ctx, current_user_id, new_messages, valid_anchor),
			process_chat_read_events(new_chat_reads),
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
			process_chat_thread_remove_events(&ctx, removed_team_ids),
		)?;

	// Merge and sort events
	let mut events = msg_events
		.into_iter()
		.chain(chat_read_events)
		.chain(user_update_events)
		.chain(mm_lobby_events)
		.chain(thread_remove_events)
		.collect::<Vec<_>>();
	// TODO: Shouldn't be sorting strings
	events.sort_by(|x, y| x.ts.cmp(&y.ts));

	Ok(models::IdentityWatchEventsResponse {
		events,
		watch: WatchResponse::new_as_model(last_update_ts),
	})
}

struct EventsWaitResponse {
	new_messages: Vec<backend::chat::Message>,

	new_chat_reads: Vec<(Uuid, backend::user::event::ChatRead)>,

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
	let mut new_messages = Vec::new();
	let mut new_chat_reads = Vec::new();
	let mut new_mm_lobby_joins = Vec::new();
	let mut removed_team_ids = Vec::new();
	let mut user_update_ts = None;
	for msg in thread_tail.messages {
		let event = internal_unwrap!(msg.event);

		if let Some(event) = &event.kind {
			match event {
				backend::user::event::event::Kind::ChatMessage(chat_msg) => {
					let chat_message = internal_unwrap!(chat_msg.chat_message).clone();
					new_messages.push(chat_message);
				}
				backend::user::event::event::Kind::ChatRead(chat_read) => {
					new_chat_reads.push((
						internal_unwrap!(chat_read.thread_id).as_uuid(),
						chat_read.clone(),
					));
				}
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
					removed_team_ids.push((msg.msg_ts(), internal_unwrap!(team.team_id).as_uuid()));
				}
			}
		} else {
			tracing::warn!(?event, "unknown user event kind");
		}
	}

	Ok(EventsWaitResponse {
		new_messages,
		new_chat_reads,
		new_mm_lobby_joins,
		removed_team_ids,
		user_update_ts,
		last_update_ts,
		valid_anchor: thread_tail.anchor_status != chirp_client::TailAllAnchorStatus::Expired,
	})
}

// MARK: Chat read events
async fn process_chat_read_events(
	new_chat_reads: Vec<(Uuid, backend::user::event::ChatRead)>,
) -> GlobalResult<Vec<models::IdentityGlobalEvent>> {
	new_chat_reads
		.into_iter()
		.map(|(thread_id, event)| {
			Ok(models::IdentityGlobalEvent {
				ts: util::timestamp::to_string(event.read_ts)?,
				kind: Box::new(models::IdentityGlobalEventKind {
					chat_read: Some(Box::new(models::IdentityGlobalEventChatRead {
						read_ts: util::timestamp::to_string(event.read_ts)?,
						thread_id,
					})),
					..Default::default()
				}),
				notification: None,
			})
		})
		.collect::<GlobalResult<Vec<_>>>()
}

// MARK: Chat message events
async fn process_msg_events(
	ctx: &Ctx<Auth>,
	current_user_id: Uuid,
	new_messages: Vec<backend::chat::Message>,
	valid_anchor: bool,
) -> GlobalResult<Vec<models::IdentityGlobalEvent>> {
	// Fetch threads
	let FetchThreadsResponse {
		threads,
		tail_messages,
	} = if valid_anchor {
		fetch_threads_incremental(ctx, current_user_id, new_messages).await?
	} else {
		fetch_threads_refresh(ctx, current_user_id).await?
	};

	if threads.is_empty() {
		return Ok(Vec::new());
	}

	let (threads, users_res) =
		fetch::chat::threads(ctx.op_ctx(), current_user_id, &threads, &tail_messages).await?;

	let mut msg_events = Vec::with_capacity(threads.len());
	for thread in threads {
		let thread_id = Some(Into::<common::Uuid>::into(thread.thread_id));
		let message =
			internal_unwrap_owned!(tail_messages.iter().find(|msg| msg.thread_id == thread_id));

		let notification = utils::create_notification(current_user_id, &users_res.users, message)?;

		msg_events.push(models::IdentityGlobalEvent {
			ts: util::timestamp::to_string(message.send_ts)?,
			kind: Box::new(models::IdentityGlobalEventKind {
				chat_message: Some(Box::new(models::IdentityGlobalEventChatMessage {
					thread: Box::new(thread),
				})),
				..Default::default()
			}),
			notification: notification.map(Box::new),
		});
	}

	Ok(msg_events)
}

struct FetchThreadsResponse {
	/// List of metadata for all threads.
	threads: Vec<backend::chat::Thread>,

	/// The last message for each of the threads. Used for channel previews.
	tail_messages: Vec<backend::chat::Message>,
}

/// Fetches all of the threads the user is a part of.
///
/// Call this when we can't infer incremental events because the anchor is
/// invalid.
async fn fetch_threads_refresh(
	ctx: &Ctx<Auth>,
	current_user_id: Uuid,
) -> GlobalResult<FetchThreadsResponse> {
	// Fetch initial list of threads
	let threads_res = op!([ctx] chat_thread_recent_for_user {
		user_id: Some(current_user_id.into()),
		after_ts: None,
	})
	.await?;

	// Get the most recent tail messages in order of timestamp desc
	let mut tail_threads = threads_res.threads;
	tail_threads.sort_by_key(|t| {
		t.tail_message
			.as_ref()
			.map(|m| m.send_ts)
			.unwrap_or_else(|| t.thread.as_ref().map(|t| t.create_ts).unwrap_or(0))
	});
	let sorted_threads_iter = tail_threads
		.iter()
		.rev()
		.take(CHAT_THREAD_HISTORY)
		.collect::<Vec<_>>();

	let threads = sorted_threads_iter
		.iter()
		.map(|t| Ok(internal_unwrap!(t.thread).clone()))
		.collect::<GlobalResult<Vec<_>>>()?;
	let tail_messages = sorted_threads_iter
		.iter()
		.map(|t| Ok(internal_unwrap!(t.tail_message).clone()))
		.collect::<GlobalResult<Vec<_>>>()?;

	Ok(FetchThreadsResponse {
		threads,
		tail_messages,
	})
}

/// Fetch all thread for the new messages that came in since the anchor.
async fn fetch_threads_incremental(
	ctx: &Ctx<Auth>,
	_current_user_id: Uuid,
	new_messages: Vec<backend::chat::Message>,
) -> GlobalResult<FetchThreadsResponse> {
	// Fetch information about the threads of the new messages

	// Get a deduplicated list of all thread IDs to fetch from the new
	// messages
	let thread_ids = new_messages
		.iter()
		.map(|message| Ok(internal_unwrap!(message.thread_id).as_uuid()))
		.collect::<GlobalResult<HashSet<_>>>()?
		.into_iter()
		.map(common::Uuid::from)
		.collect::<Vec<_>>();

	let threads = if !thread_ids.is_empty() {
		let threads_res = op!([ctx] chat_thread_get {
			thread_ids: thread_ids.clone(),
		})
		.await?;

		threads_res.threads
	} else {
		Vec::new()
	};

	Ok(FetchThreadsResponse {
		threads,
		tail_messages: new_messages,
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
	let profile = unwrap_with_owned!(identities.into_iter().next(), IDENTITY_NOT_FOUND);

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

// MARK: Chat thread remove events
async fn process_chat_thread_remove_events(
	ctx: &Ctx<Auth>,
	removed_team_ids: Vec<(i64, Uuid)>,
) -> GlobalResult<Vec<models::IdentityGlobalEvent>> {
	// Ignore if no removed_team_ids provided
	if removed_team_ids.is_empty() {
		return Ok(Vec::new());
	}

	let team_thread_res = op!([ctx] chat_thread_get_for_topic {
		topics: removed_team_ids
			.iter()
			.map(|(_, team_id)| backend::chat::Topic {
				kind: Some(backend::chat::topic::Kind::Team(backend::chat::topic::Team {
					team_id: Some((*team_id).into()),
				})),
			})
			.collect::<Vec<_>>(),
	})
	.await?;

	// Infallibly find matching threads
	let team_threads = removed_team_ids
		.iter()
		.map(|(ts, team_id)| (ts, Into::<common::Uuid>::into(*team_id)))
		.filter_map(|(ts, team_id)| {
			team_thread_res
				.threads
				.iter()
				.find_map(|thread| {
					match thread.topic.as_ref().and_then(|topic| topic.kind.as_ref()) {
						Some(backend::chat::topic::Kind::Team(team))
							if team.team_id == Some(team_id) =>
						{
							thread.thread_id
						}
						_ => None,
					}
				})
				.map(|thread_id| (ts, thread_id))
		});

	team_threads
		.map(|(ts, thread_id)| {
			Ok(models::IdentityGlobalEvent {
				ts: util::timestamp::to_string(*ts)?,
				kind: Box::new(models::IdentityGlobalEventKind {
					chat_thread_remove: Some(Box::new(
						models::IdentityGlobalEventChatThreadRemove {
							thread_id: thread_id.as_uuid(),
						},
					)),
					..Default::default()
				}),
				notification: None,
			})
		})
		.collect::<GlobalResult<Vec<_>>>()
}

async fn fetch_lobby(
	ctx: &OperationContext<()>,
	lobby: &backend::matchmaker::Lobby,
	player_token: &str,
) -> GlobalResult<models::IdentityGlobalEventMatchmakerLobbyJoin> {
	let lobby_id = internal_unwrap!(lobby.lobby_id).as_uuid();
	let region_id = internal_unwrap!(lobby.region_id);
	let lobby_group_id = internal_unwrap!(lobby.lobby_group_id);
	let run_id = internal_unwrap!(lobby.run_id);

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

			let version_id = internal_unwrap_owned!(version_res.versions.first());
			let version_id = internal_unwrap!(version_id.version_id);
			let version_res = op!([ctx] mm_config_version_get {
				version_ids: vec![*version_id],
			})
			.await?;
			let version = internal_unwrap_owned!(version_res.versions.first());

			GlobalResult::Ok(version.clone())
		}
	)?;

	// Match the version
	let version_config = internal_unwrap!(version.config);
	let version_meta = internal_unwrap!(version.config_meta);
	let (lobby_group_config, _lobby_group_meta) = internal_unwrap_owned!(version_config
		.lobby_groups
		.iter()
		.zip(version_meta.lobby_groups.iter())
		.find(|(_, meta)| meta.lobby_group_id.as_ref() == Some(lobby_group_id)));
	let lobby_runtime = internal_unwrap!(lobby_group_config.runtime);
	#[allow(clippy::infallible_destructuring_match)]
	let docker_runtime = match internal_unwrap!(lobby_runtime.runtime) {
		backend::matchmaker::lobby_runtime::Runtime::Docker(x) => x,
	};

	// Convert the ports to client-friendly ports
	let run = internal_unwrap_owned!(run_res.runs.first());

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
	let region_proto = internal_unwrap_owned!(region_res.regions.first());
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
	use backend::job::ProxyProtocol as JobProxyProtocol;
	use backend::matchmaker::lobby_runtime::{
		ProxyKind as MmProxyKind, ProxyProtocol as MmProxyProtocol,
	};

	let proxy_kind = internal_unwrap_owned!(MmProxyKind::from_i32(port.proxy_kind));
	let mm_proxy_protocol = internal_unwrap_owned!(MmProxyProtocol::from_i32(port.proxy_protocol));

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
			let port_range = internal_unwrap!(port.port_range);

			let network = internal_unwrap_owned!(
				run.networks.iter().find(|x| x.mode == "host"),
				"missing host network"
			);

			Some(models::MatchmakerJoinPort {
				host: None,
				hostname: network.ip.clone(),
				port: None,
				port_range: Some(Box::new(models::MatchmakerJoinPortRange {
					min: port_range.min.try_into()?,
					max: port_range.max.try_into()?,
				})),
				is_tls: false,
			})
		}
		(
			MmProxyKind::None,
			MmProxyProtocol::Http | MmProxyProtocol::Https | MmProxyProtocol::TcpTls,
		) => {
			internal_panic!("invalid http proxy protocol with host network")
		}
	};

	GlobalResult::Ok(join_info_port.map(|x| (port.label.clone(), x)))
}

fn test_mm_and_job_proxy_protocol_eq(
	mm_proxy_protocol: backend::matchmaker::lobby_runtime::ProxyProtocol,
	job_proxy_protocol: backend::job::ProxyProtocol,
) -> bool {
	use backend::job::ProxyProtocol as JPP;
	use backend::matchmaker::lobby_runtime::ProxyProtocol as MPP;

	match (mm_proxy_protocol, job_proxy_protocol) {
		(MPP::Http, JPP::Http) => true,
		(MPP::Https, JPP::Https) => true,
		(MPP::Tcp, JPP::Tcp) => true,
		(MPP::TcpTls, JPP::TcpTls) => true,
		(MPP::Udp, JPP::Udp) => true,
		_ => false,
	}
}
