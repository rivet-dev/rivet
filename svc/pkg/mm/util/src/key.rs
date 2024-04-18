use uuid::Uuid;

use crate::JoinKind;

/// HASH
pub fn player_config(player_id: Uuid) -> String {
	format!("{{global}}:mm:player:{}:config", player_id)
}

pub mod player_config {
	use uuid::Uuid;

	#[derive(Debug, serde::Serialize)]
	pub struct Config {
		#[serde(rename = "l")]
		pub lobby_id: Uuid,
		#[serde(rename = "qi")]
		pub query_id: Option<Uuid>,
		#[serde(rename = "ra")]
		pub remote_address: String,
	}

	pub const LOBBY_ID: &str = "l";
	pub const QUERY_ID: &str = "qi";
	pub const REMOTE_ADDRESS: &str = "ra";
}

/// HASH
pub fn lobby_config(lobby_id: Uuid) -> String {
	format!("{{global}}:mm:lobby:{}:config", lobby_id)
}

/// HASH
pub fn lobby_tags(lobby_id: Uuid) -> String {
	format!("{{global}}:mm:lobby:{}:tags", lobby_id)
}

pub mod lobby_config {
	use uuid::Uuid;

	#[derive(Debug, serde::Serialize)]
	pub struct Config {
		#[serde(rename = "ns")]
		pub namespace_id: Uuid,
		#[serde(rename = "r")]
		pub region_id: Uuid,
		#[serde(rename = "lg")]
		pub lobby_group_id: Uuid,
		#[serde(rename = "mpn")]
		pub max_players_normal: u32,
		#[serde(rename = "mpp")]
		pub max_players_party: u32,
		#[serde(rename = "mpd")]
		pub max_players_direct: u32,
		#[serde(rename = "p")]
		pub preemptive: bool,
		#[serde(rename = "rt", skip_serializing_if = "Option::is_none")]
		pub ready_ts: Option<i64>,
		#[serde(rename = "c")]
		pub is_closed: bool,
		#[serde(rename = "cu")]
		pub is_custom: bool,
		#[serde(rename = "st", skip_serializing_if = "Option::is_none")]
		pub state_json: Option<String>,
	}

	pub const NAMESPACE_ID: &str = "ns";
	pub const REGION_ID: &str = "r";
	pub const LOBBY_GROUP_ID: &str = "lg";
	pub const MAX_PLAYERS_NORMAL: &str = "mpn";
	pub const MAX_PLAYERS_PARTY: &str = "mpp";
	pub const MAX_PLAYERS_DIRECT: &str = "mpd";
	pub const PREEMPTIVE: &str = "p";
	pub const READY_TS: &str = "rt";
	pub const IS_CLOSED: &str = "c";
	pub const IS_CUSTOM: &str = "cu";
	pub const STATE_JSON: &str = "st";
}

/// HASH
///
/// Includes the state of all active find queries.
pub fn find_query_state(query_id: Uuid) -> String {
	format!("{{global}}:mm:find_query:{}:state", query_id)
}

pub mod find_query_state {
	use uuid::Uuid;

	#[derive(Debug, serde::Serialize)]
	pub struct State {
		#[serde(rename = "n")]
		pub namespace_id: Uuid,
		#[serde(rename = "l", skip_serializing_if = "Option::is_none")]
		pub lobby_id: Option<Uuid>,
		#[serde(rename = "lac", skip_serializing_if = "Option::is_none")]
		pub lobby_auto_created: Option<bool>,
		#[serde(rename = "s")]
		pub status: u8,
	}

	pub const NAMESPACE_ID: &str = "n";
	pub const PLAYER_IDS: &str = "pl";
	pub const LOBBY_ID: &str = "l";
	pub const LOBBY_AUTO_CREATED: &str = "lac";
	pub const STATUS: &str = "s";
}

/// SET<player id>
pub fn find_query_player_ids(query_id: Uuid) -> String {
	format!("{{global}}:mm:find_query:{}:player_ids", query_id)
}

/// ZSET<create ts, query id>
///
/// Includes all active find queries for a lobby.
pub fn lobby_find_queries(lobby_id: Uuid) -> String {
	format!("{{global}}:mm:lobby:{}:find_queries", lobby_id)
}

/// ZSET<player id>
pub fn ns_player_ids(namespace_id: Uuid) -> String {
	format!("{{global}}:mm:ns:{}:player_ids", namespace_id)
}

/// ZSET<lobby id>
pub fn ns_lobby_ids(namespace_id: Uuid) -> String {
	format!("{{global}}:mm:ns:{}:lobby_ids", namespace_id)
}

/// SET<player id>
pub fn ns_remote_address_player_ids(namespace_id: Uuid, remote_address: &str) -> String {
	format!(
		"{{global}}:mm:ns:{}:remote_address:{}:player_ids",
		namespace_id, remote_address
	)
}

/// ZSET<player id>
pub fn lobby_player_ids(lobby_id: Uuid) -> String {
	format!("{{global}}:mm:lobby:{}:player_ids", lobby_id)
}

/// ZSET<player id>
pub fn lobby_registered_player_ids(lobby_id: Uuid) -> String {
	format!("{{global}}:mm:lobby:{}:registered_player_ids", lobby_id)
}

/// ZSET<lobby id, idle ts>
pub fn idle_lobby_ids(namespace_id: Uuid, region_id: Uuid, lobby_group_id: Uuid) -> String {
	format!(
		"{{global}}:mm:ns:{}:region:{}:lg:{}:idle_lobby_ids",
		namespace_id, region_id, lobby_group_id
	)
}

/// Map containing all idle lobbies and their associated lobby group
/// IDs.
///
/// We limit this to just idle lobbies since we need to iterate over all
/// the values in this hash in mm-lobby-idle-update, so we want to limit
/// the values in here as much as possible.
///
/// We keep this all in one hash so we only have to lock one key instead
/// of using `SCAN`.
///
/// HASH<lobby id, lobby group id>
pub fn idle_lobby_lobby_group_ids(namespace_id: Uuid, region_id: Uuid) -> String {
	format!(
		"{{global}}:mm:ns:{}:region:{}:lobby:idle:lobby_group_ids",
		namespace_id, region_id,
	)
}

/// ZSET<lobby id, available spots>
pub fn lobby_available_spots(
	namespace_id: Uuid,
	region_id: Uuid,
	lobby_group_id: Uuid,
	join_kind: JoinKind,
) -> String {
	format!(
		"{{global}}:mm:ns:{}:region:{}:lg:{}:lobby:available_spots:{}",
		namespace_id,
		region_id,
		lobby_group_id,
		join_kind.short()
	)
}

/// ZSET<lobby id, expire ts>
pub fn lobby_unready() -> String {
	"{global}:mm:lobby:unready".to_string()
}

/// ZSET<lobby id, expire ts>
pub fn player_unregistered() -> String {
	"{global}:mm:player:unregistered".to_string()
}

/// ZSET<lobby id, expire ts>
pub fn player_auto_remove() -> String {
	"{global}:mm:player:auto_remove".to_string()
}

/// is closed
pub fn node_is_closed(
	node_id: &str,
) -> String {
	format!(
		"{{global}}:mm:node:{}:is_closed",
		node_id,
	)
}

// Placeholder key
pub fn empty() -> String {
	"{global}".to_string()
}
