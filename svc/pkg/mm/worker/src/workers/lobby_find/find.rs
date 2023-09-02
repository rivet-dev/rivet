use chirp_worker::prelude::*;
use proto::backend::{
	self,
	matchmaker::query::JoinKind,
	pkg::{
		mm::{msg::lobby_find::message::Query, msg::lobby_find_fail::ErrorCode},
		*,
	},
};
use rand::seq::SliceRandom;
use redis_util::RedisResult;

use super::{common, fail, LobbyGroupConfig};

lazy_static::lazy_static! {
	/// Finds the optimal lobby and optimally creates a new lobby if needed.
	static ref REDIS_SCRIPT: redis::Script = redis::Script::new(include_str!("../../../redis-scripts/lobby_find.lua"));
}

/// Query that will be passed to the Redis find script as JSON.
mod redis_query {
	use chirp_worker::prelude::*;
	use serde::Serialize;

	#[derive(Serialize)]
	pub struct Query {
		pub query_id: Uuid,

		/// State that will be inserted for this find query.
		pub find_query_state: util_mm::key::find_query_state::State,

		/// Information about what lobby to search for.
		pub kind: QueryKind,

		/// Context about how this lobby is being joined.
		pub join_kind: JoinKind,

		/// Players to insert.
		pub players: Vec<Player>,

		pub player_register_expire_ts: i64,
	}

	#[derive(Serialize)]
	pub enum QueryKind {
		#[serde(rename = "direct")]
		Direct { lobby_id: Uuid },
		#[serde(rename = "lobby_group")]
		LobbyGroup {
			#[serde(skip_serializing_if = "Option::is_none")]
			auto_create: Option<AutoCreate>,
		},
	}

	#[derive(Serialize)]
	#[serde(rename_all = "camelCase")]
	pub enum JoinKind {
		Normal,
		Direct,
		Party,
	}

	#[derive(Serialize)]
	pub struct AutoCreate {
		/// The lobby for this ID does not exist yet. We will call
		/// msg-mm-lobby-create after this.
		pub lobby_id: Uuid,
		pub lobby_config: util_mm::key::lobby_config::Config,
		pub ready_expire_ts: i64,
	}

	#[derive(Serialize)]
	pub struct Player {
		pub player_id: Uuid,
		#[serde(skip_serializing_if = "Option::is_none")]
		pub remote_address: Option<String>,
	}
}

pub struct FindOpts<'a> {
	pub namespace_id: Uuid,
	pub query_id: Uuid,
	pub join_kind: JoinKind,
	pub players: &'a [super::Player],
	pub query: &'a Query,
	pub lobby_group_config: &'a LobbyGroupConfig,
	pub auto_create_lobby_id: Uuid,
}

pub struct FindOutput {
	pub lobby_id: Uuid,
	pub region_id: Uuid,
	pub lobby_group_id: Uuid,
}

/// Finds or creates a lobby for the given query.
///
/// Returns `None` if called `fail`.
#[tracing::instrument(skip(crdb, redis_mm))]
pub async fn find(
	ctx: &OperationContext<mm::msg::lobby_find::Message>,
	crdb: &CrdbPool,
	redis_mm: &mut RedisConn,
	FindOpts {
		namespace_id,
		query_id,
		join_kind,
		players,
		query,
		lobby_group_config,
		auto_create_lobby_id,
	}: FindOpts<'_>,
) -> GlobalResult<Option<FindOutput>> {
	use util_mm::key;

	// Build query config.
	//
	// We assemble all `*_keys` separately so we can determine the indices to
	// pass to the Lua script programmatically. This way we don't have to
	// manually write & update `KEYS` indices within the script.
	let mut query_kind_keys = Vec::new();
	let mut available_spots_keys = Vec::new();
	let mut player_config_keys = Vec::new();
	let mut ns_remote_address_player_ids_keys = Vec::new();

	let (redis_query_kind, join_kind) = match query {
		Query::Direct(backend::matchmaker::query::Direct { lobby_id }) => {
			let lobby_id = internal_unwrap!(lobby_id).as_uuid();

			// Add keys for lobby
			query_kind_keys.extend([key::lobby_config(lobby_id), key::lobby_player_ids(lobby_id)]);

			(
				redis_query::QueryKind::Direct { lobby_id },
				match join_kind {
					JoinKind::Normal => util_mm::JoinKind::Direct,
					JoinKind::Party => util_mm::JoinKind::Party,
				},
			)
		}
		Query::LobbyGroup(backend::matchmaker::query::LobbyGroup {
			lobby_group_ids,
			region_ids,
			auto_create,
		}) => {
			let lobby_group_ids = lobby_group_ids
				.iter()
				.map(common::Uuid::as_uuid)
				.collect::<Vec<_>>();
			let region_ids = region_ids
				.iter()
				.map(common::Uuid::as_uuid)
				.collect::<Vec<_>>();

			// Update config for auto create lobbies
			let auto_create = if let Some(auto_create) = auto_create {
				let region_id = internal_unwrap!(auto_create.region_id).as_uuid();
				let lobby_group_id = internal_unwrap!(auto_create.lobby_group_id).as_uuid();

				// Add keys for auto creating lobby
				query_kind_keys.extend([
					key::lobby_unready(),
					key::lobby_config(auto_create_lobby_id),
					key::ns_lobby_ids(namespace_id),
					key::lobby_available_spots(
						namespace_id,
						region_id,
						lobby_group_id,
						util_mm::JoinKind::Normal,
					),
					key::lobby_available_spots(
						namespace_id,
						region_id,
						lobby_group_id,
						util_mm::JoinKind::Party,
					),
				]);

				// Create lobby config
				Some(redis_query::AutoCreate {
					lobby_id: auto_create_lobby_id,
					lobby_config: util_mm::key::lobby_config::Config {
						namespace_id,
						region_id,
						lobby_group_id,
						max_players_normal: lobby_group_config.lobby_group.max_players_normal,
						max_players_party: lobby_group_config.lobby_group.max_players_party,
						max_players_direct: lobby_group_config.lobby_group.max_players_direct,
						preemptive: true,
						ready_ts: None,
						is_closed: false,
						is_custom: false,
						state_json: None,
					},
					ready_expire_ts: ctx.ts() + util_mm::consts::LOBBY_READY_TIMEOUT,
				})
			} else {
				None
			};

			// Determine keys to find lobbies in for lobby groups.
			//
			// Earlier keys are prioritized over later keys.
			//
			// The keys are randomized in order to ensure fair prioritization of
			// lobby groups.
			for region_id in &region_ids {
				for lobby_group_id in &lobby_group_ids {
					available_spots_keys.push(key::lobby_available_spots(
						namespace_id,
						*region_id,
						*lobby_group_id,
						util_mm::JoinKind::Normal,
					));
				}
			}
			available_spots_keys.shuffle(&mut rand::thread_rng());

			(
				redis_query::QueryKind::LobbyGroup { auto_create },
				match join_kind {
					JoinKind::Normal => util_mm::JoinKind::Normal,
					JoinKind::Party => util_mm::JoinKind::Party,
				},
			)
		}
	};

	// Configure players to add
	let mut query_players = Vec::new();
	for player in players {
		let remote_address = player
			.client_info
			.as_ref()
			.and_then(|x| x.remote_address.as_ref());

		query_players.push(redis_query::Player {
			player_id: player.player_id,
			remote_address: remote_address.cloned(),
		});

		// Add keys to write players to
		player_config_keys.push(util_mm::key::player_config(player.player_id));
		ns_remote_address_player_ids_keys.push(if let Some(remote_address) = remote_address {
			util_mm::key::ns_remote_address_player_ids(namespace_id, remote_address)
		} else {
			String::new()
		});
	}

	// Merge keys and determine key indices
	let mut keys = vec![
		util_mm::key::find_query_state(query_id),
		util_mm::key::find_query_player_ids(query_id),
		util_mm::key::ns_player_ids(namespace_id),
		util_mm::key::player_unregistered(),
	];

	let query_kind_key_idx = keys.len();
	keys.extend(query_kind_keys);

	let available_spots_key_idx = keys.len();
	let available_spots_key_count = available_spots_keys.len();
	keys.extend(available_spots_keys);

	let player_config_key_idx = keys.len();
	keys.extend(player_config_keys);

	let ns_remote_address_player_ids_key_idx = keys.len();
	keys.extend(ns_remote_address_player_ids_keys);

	// Build query struct
	let redis_query = redis_query::Query {
		query_id,
		find_query_state: util_mm::key::find_query_state::State {
			namespace_id,
			lobby_id: None,
			lobby_auto_created: None,
			status: util_mm::FindQueryStatus::Pending as u8,
		},
		kind: redis_query_kind,
		join_kind: match join_kind {
			util_mm::JoinKind::Normal => redis_query::JoinKind::Normal,
			util_mm::JoinKind::Party => redis_query::JoinKind::Party,
			util_mm::JoinKind::Direct => redis_query::JoinKind::Direct,
		},
		players: query_players,
		player_register_expire_ts: ctx.ts() + util_mm::consts::PLAYER_READY_TIMEOUT,
	};

	// Execute script
	let mut script_find = REDIS_SCRIPT.prepare_invoke();
	script_find
		.arg(ctx.ts())
		.arg(&serde_json::to_string(&redis_query)?)
		.arg(query_kind_key_idx)
		.arg(available_spots_key_idx)
		.arg(available_spots_key_count)
		.arg(player_config_key_idx)
		.arg(ns_remote_address_player_ids_key_idx);
	for key in keys {
		script_find.key(key);
	}
	let redis_res = script_find
		.invoke_async::<_, RedisResult<(String, String, String)>>(redis_mm)
		.await?;

	// Handle result
	let error_code = match redis_res.as_ref().map_err(String::as_str) {
		Ok((lobby_id, region_id, lobby_group_id)) => {
			let lobby_id = util::uuid::parse(lobby_id)?;
			let region_id = util::uuid::parse(region_id)?;
			let lobby_group_id = util::uuid::parse(lobby_group_id)?;
			return Ok(Some(FindOutput {
				lobby_id,
				region_id,
				lobby_group_id,
			}));
		}
		Err("LOBBY_NOT_FOUND") => {
			// Check if lobby was already stopped when joining directly
			if let Query::Direct(backend::matchmaker::query::Direct { lobby_id }) = query {
				let lobby_id = internal_unwrap!(lobby_id).as_uuid();

				if let Some((Some(_),)) = sqlx::query_as::<_, (Option<i64>,)>(
					"SELECT stop_ts FROM lobbies WHERE lobby_id = $1",
				)
				.bind(lobby_id)
				.fetch_optional(crdb)
				.await?
				{
					ErrorCode::LobbyStopped
				} else {
					ErrorCode::LobbyNotFound
				}
			} else {
				ErrorCode::LobbyNotFound
			}
		}
		Err("LOBBY_CLOSED") => ErrorCode::LobbyClosed,
		Err("LOBBY_FULL") => ErrorCode::LobbyFull,
		Err("NO_AVAILABLE_LOBBIES") => ErrorCode::NoAvailableLobbies,
		Err(_) => internal_panic!("unknown redis error"),
	};

	fail(ctx, namespace_id, query_id, error_code, true)
		.await
		.map(|_| None)
}
