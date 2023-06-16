#[allow(unused_imports)]
use chrono;
#[allow(unused_imports)]
use serde::{Deserialize, Serialize};

/// Game region statistics.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RegionStatistics {
	/// Unsigned 64 bit integer.
	pub player_count: i64,
	#[allow(missing_docs)] // documentation missing in model
	pub game_modes:
		std::collections::HashMap<std::string::String, GameModeStatistics>,
}

/// Game mode statistics
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GameModeStatistics {
	/// Unsigned 64 bit integer.
	pub player_count: i64,
}

/// Game namespace statistics.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct NamespaceStatistics {
	/// Unsigned 64 bit integer.
	pub player_count: i64,
	#[allow(missing_docs)] // documentation missing in model
	pub game_modes:
		std::collections::HashMap<std::string::String, GameModeStatistics>,
}

/// A region that the player can connect to.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RegionInfo {
	/// A human readable short identifier used to references resources. Different than a `rivet.common#Uuid` because this is intended to be human readable. Different than `rivet.common#DisplayName` because this should not include special characters and be short.
	pub region_id: std::string::String,
	/// A universally unique identifier.
	pub provider_display_name: std::string::String,
	/// A universally unique identifier.
	pub region_display_name: std::string::String,
	/// Geographical coordinates for a location on Planet Earth.
	pub datacenter_coord: Coord,
	/// Distance available in multiple units.
	pub datacenter_distance_from_client: Distance,
}

/// Distance available in multiple units.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Distance {
	#[allow(missing_docs)] // documentation missing in model
	pub kilometers: f64,
	#[allow(missing_docs)] // documentation missing in model
	pub miles: f64,
}

/// Geographical coordinates for a location on Planet Earth.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Coord {
	#[allow(missing_docs)] // documentation missing in model
	pub latitude: f64,
	#[allow(missing_docs)] // documentation missing in model
	pub longitude: f64,
}

/// A public lobby in the lobby list.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct LobbyInfo {
	#[allow(missing_docs)] // documentation missing in model
	pub region_id: std::string::String,
	#[allow(missing_docs)] // documentation missing in model
	pub game_mode_id: std::string::String,
	/// A universally unique identifier.
	pub lobby_id: std::string::String,
	/// Unsigned 32 bit integer.
	pub max_players_normal: i32,
	/// Unsigned 32 bit integer.
	pub max_players_direct: i32,
	/// Unsigned 32 bit integer.
	pub max_players_party: i32,
	/// Unsigned 32 bit integer.
	pub total_player_count: i32,
}

/// A game mode that the player can join.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GameModeInfo {
	/// A human readable short identifier used to references resources. Different than a `rivet.common#Uuid` because this is intended to be human readable. Different than `rivet.common#DisplayName` because this should not include special characters and be short.
	pub game_mode_id: std::string::String,
}

/// A matchmaker lobby.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct MatchmakerLobbyJoinInfo {
	/// A universally unique identifier.
	pub lobby_id: std::string::String,
	/// A matchmaker lobby region.
	pub region: MatchmakerLobbyJoinInfoRegion,
	/// A list of lobby ports.
	pub ports:
		std::collections::HashMap<std::string::String, MatchmakerLobbyJoinInfoPort>,
	/// A matchmaker lobby player.
	pub player: MatchmakerLobbyJoinInfoPlayer,
}

/// A matchmaker lobby player.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct MatchmakerLobbyJoinInfoPlayer {
	/// Pass this token through the socket to the lobby server. The lobby server will validate this token with `rivet.api.matchmaker#PlayerConnected$player_token`.
	pub token: std::string::String,
}

/// A matchmaker lobby port. Configured by `rivet.cloud#LobbyGroupRuntimeDockerPort$label`.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct MatchmakerLobbyJoinInfoPort {
	/// The host for the given port. Will be null if using a port range.
	pub host: std::option::Option<std::string::String>,
	/// The hostname for the given port.
	pub hostname: std::string::String,
	/// The port number for this lobby. Will be null if using a port range.
	pub port: std::option::Option<i32>,
	/// The port range for this lobby.
	pub port_range: std::option::Option<MatchmakerLobbyJoinInfoPortRange>,
	/// Wether or not this lobby port uses TLS. You cannot mix a non-TLS and TLS ports.
	pub is_tls: bool,
}

/// Inclusive range of ports that can be connected to.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct MatchmakerLobbyJoinInfoPortRange {
	/// Minimum port that can be connected to. Inclusive range.
	pub min: i32,
	/// Maximum port that can be connected to. Inclusive range.
	pub max: i32,
}

/// A matchmaker lobby region.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct MatchmakerLobbyJoinInfoRegion {
	/// A human readable short identifier used to references resources. Different than a `rivet.common#Uuid` because this is intended to be human readable. Different than `rivet.common#DisplayName` because this should not include special characters and be short.
	pub region_id: std::string::String,
	/// Represent a resource's readable display name.
	pub display_name: std::string::String,
}

/// Methods to verify a captcha.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CaptchaConfig {
	/// hCaptcha configuration.
	Hcaptcha(CaptchaConfigHcaptcha),
	/// Cloudflare Turnstile configuration.
	Turnstile(CaptchaConfigTurnstile),
}

/// Cloudflare Turnstile configuration.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CaptchaConfigTurnstile {
	#[allow(missing_docs)] // documentation missing in model
	pub client_response: std::string::String,
}

/// hCaptcha configuration.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CaptchaConfigHcaptcha {
	#[allow(missing_docs)] // documentation missing in model
	pub client_response: std::string::String,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetGameStatisticsRequest {}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ListRegionsRequest {}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PlayerDisconnectedRequest {
	/// A JSON Web Token. Slightly modified to include a description prefix and use Protobufs of JSON.
	pub player_token: std::string::String,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PlayerConnectedRequest {
	/// A JSON Web Token. Slightly modified to include a description prefix and use Protobufs of JSON.
	pub player_token: std::string::String,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SetLobbyClosedRequest {
	#[allow(missing_docs)] // documentation missing in model
	pub is_closed: bool,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ListLobbiesRequest {}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct FindLobbyRequest {
	/// Game modes to match lobbies against.
	pub game_modes: std::vec::Vec<std::string::String>,
	/// Regions to match lobbies against. If not specified, the optimal region will be determined and will attempt to find lobbies in that region.
	pub regions: std::option::Option<std::vec::Vec<std::string::String>>,
	/// Prevents a new lobby from being created when finding a lobby. If no lobby is found, a `MATCHMAKER_LOBBY_NOT_FOUND` error will be thrown.
	pub prevent_auto_create_lobby: std::option::Option<bool>,
	/// Methods to verify a captcha.
	pub captcha: std::option::Option<CaptchaConfig>,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct JoinLobbyRequest {
	/// A universally unique identifier.
	pub lobby_id: std::string::String,
	/// Methods to verify a captcha.
	pub captcha: std::option::Option<CaptchaConfig>,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct LobbyReadyRequest {}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetGameStatisticsResponse {
	/// Unsigned 64 bit integer.
	pub player_count: i64,
	#[allow(missing_docs)] // documentation missing in model
	pub namespaces:
		std::collections::HashMap<std::string::String, NamespaceStatistics>,
	#[allow(missing_docs)] // documentation missing in model
	pub regions: std::collections::HashMap<std::string::String, RegionStatistics>,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ListRegionsResponse {
	#[allow(missing_docs)] // documentation missing in model
	pub regions: std::vec::Vec<RegionInfo>,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PlayerDisconnectedResponse {}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PlayerConnectedResponse {}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SetLobbyClosedResponse {}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ListLobbiesResponse {
	#[allow(missing_docs)] // documentation missing in model
	pub game_modes: std::vec::Vec<GameModeInfo>,
	#[allow(missing_docs)] // documentation missing in model
	pub regions: std::vec::Vec<RegionInfo>,
	#[allow(missing_docs)] // documentation missing in model
	pub lobbies: std::vec::Vec<LobbyInfo>,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct FindLobbyResponse {
	/// A matchmaker lobby.
	pub lobby: MatchmakerLobbyJoinInfo,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct JoinLobbyResponse {
	/// A matchmaker lobby.
	pub lobby: MatchmakerLobbyJoinInfo,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct LobbyReadyResponse {}

