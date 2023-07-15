use serde::Serialize;
use uuid::Uuid;

#[derive(Serialize)]
pub struct ExternalVerificationRequest {
	pub verification_data: Option<serde_json::Value>,
	pub lobby: Lobby,
	#[serde(rename = "type")]
	pub _type: MatchmakerConnectionType,
}

#[derive(Serialize)]
pub struct Lobby {
	pub namespace_id: Uuid,
	pub lobby_group_id: Uuid,
	pub lobby_group_name_id: String,
	
	pub state: Option<LobbyState>,
	pub config: Option<serde_json::Value>,
}

#[derive(Serialize)]
pub struct LobbyState {
	pub lobby_id: Uuid,
	pub region_id: Uuid,
	pub region_name_id: String,
	pub create_ts: String,
	pub is_closed: bool,
}

#[derive(Serialize)]
pub enum MatchmakerConnectionType {
	Find,
	Join,
	Create
}
