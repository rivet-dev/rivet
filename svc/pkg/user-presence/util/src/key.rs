use uuid::Uuid;

/// HASH
pub fn user_presence(user_id: Uuid) -> String {
	format!("{{global}}:user_presence:user:{user_id}")
}

pub mod user_presence {
	use serde::{Deserialize, Serialize};
	use uuid::Uuid;

	#[derive(Debug, Serialize, Deserialize)]
	pub struct State {
		#[serde(rename = "u")]
		pub user_id: Uuid,
		#[serde(rename = "ut")]
		pub update_ts: i64,
		#[serde(rename = "s")]
		pub status: i64,
	}

	pub const USER_ID: &str = "u";
	pub const UPDATE_TS: &str = "ut";
	pub const STATUS: &str = "s";
}

/// HASH
pub fn game_activity(user_id: Uuid) -> String {
	format!("{{global}}:user_presence:game_activity:{user_id}")
}

pub mod game_activity {
	use serde::{Deserialize, Serialize};
	use uuid::Uuid;

	#[derive(Debug, Serialize, Deserialize)]
	pub struct State {
		#[serde(rename = "u")]
		pub user_id: Uuid,
		#[serde(rename = "g")]
		pub game_id: Uuid,
		#[serde(rename = "ut")]
		pub update_ts: i64,
		#[serde(rename = "m")]
		pub message: String,
		#[serde(rename = "pm")]
		pub public_metadata_json: Option<String>,
		#[serde(rename = "fm")]
		pub friend_metadata_json: Option<String>,
	}

	pub const USER_ID: &str = "u";
	pub const GAME_ID: &str = "g";
	pub const UPDATE_TS: &str = "ut";
	pub const MESSAGE: &str = "m";
	pub const PUBLIC_METADATA_JSON: &str = "pm";
	pub const FRIEND_METADATA_JSON: &str = "gm";
}

/// ZSET
pub fn user_presence_touch() -> String {
	"{global}:user_presence:user:touch".to_string()
}
