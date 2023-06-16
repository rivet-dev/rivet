use uuid::Uuid;

// MARK: Party
/// HASH
pub fn party_config(party_id: Uuid) -> String {
	format!("party:party:config:{party_id}")
}

pub mod party_config {
	use serde::{Deserialize, Serialize};
	use uuid::Uuid;

	#[derive(Debug, Serialize, Deserialize)]
	pub struct Config {
		pub party_id: Uuid,
		pub create_ts: i64,
		pub leader_user_id: Option<Uuid>,
		pub party_size: u32,
		pub state_change_ts: i64,
		pub state: State,
		pub publicity: Publicity,
	}

	#[derive(Debug, Serialize, Deserialize)]
	#[serde(rename_all = "snake_case")]
	pub enum State {
		Idle {},
		MatchmakerFindingLobby { namespace_id: Uuid, query_id: Uuid },
		MatchmakerLobby { namespace_id: Uuid, lobby_id: Uuid },
	}

	#[derive(Debug, Serialize, Deserialize)]
	#[serde(rename_all = "snake_case")]
	pub struct Publicity {
		pub public: PublicityLevel,
		pub friends: PublicityLevel,
		pub teams: PublicityLevel,
	}

	impl Default for Publicity {
		fn default() -> Self {
			Self {
				public: PublicityLevel::View,
				friends: PublicityLevel::Join,
				teams: PublicityLevel::View,
			}
		}
	}

	#[derive(Debug, Serialize, Deserialize)]
	#[serde(rename_all = "snake_case")]
	pub enum PublicityLevel {
		None,
		View,
		Join,
	}
}

// MARK: Party member
/// HASH
pub fn party_member_config(user_id: Uuid) -> String {
	format!("party:member:config:{user_id}")
}

pub mod party_member_config {
	use serde::{Deserialize, Serialize};
	use uuid::Uuid;

	#[derive(Debug, Serialize, Deserialize)]
	pub struct Config {
		pub party_id: Uuid,
		pub user_id: Uuid,
		pub create_ts: i64,
		pub state_change_ts: i64,
		pub state: State,
		pub client_info: Option<ClientInfo>,
	}

	#[derive(Debug, Serialize, Deserialize)]
	pub struct ClientInfo {
		pub user_agent: Option<String>,
		pub remote_address: Option<String>,
	}

	#[derive(Debug, Serialize, Deserialize)]
	#[serde(rename_all = "snake_case")]
	pub enum State {
		Inactive {},
		MatchmakerReady {},
		MatchmakerFindingLobby {
			player_id: Uuid,
			player_token: String,
		},
		MatchmakerFindingLobbyDirect {
			direct_query_id: Uuid,
			player_id: Uuid,
			player_token: Option<String>,
		},
		MatchmakerLobby {
			player_id: Uuid,
			player_token: String,
		},
	}
}

// MARK: Party invite
/// HASH
pub fn party_invite_config(invite_id: Uuid) -> String {
	format!("party:invite:config:{invite_id}")
}

pub mod party_invite_config {
	use serde::{Deserialize, Serialize};
	use uuid::Uuid;

	#[derive(Debug, Serialize, Deserialize)]
	pub struct Config {
		pub invite_id: Uuid,
		pub party_id: Uuid,
		pub create_ts: i64,
		pub token: String,
		pub alias: Option<Alias>,
	}

	#[derive(Debug, Serialize, Deserialize)]
	pub struct Alias {
		pub namespace_id: Uuid,
		pub alias: String,
	}

	pub const PARTY_ID: &str = "pi";
	pub const CREATE_TS: &str = "ct";
	pub const TOKEN: &str = "t";
	pub const ALIAS_NAMESPACE_ID: &str = "ani";
	pub const ALIAS: &str = "a";
}
