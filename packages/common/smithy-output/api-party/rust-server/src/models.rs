#[allow(unused_imports)]
use chrono;
#[allow(unused_imports)]
use serde::{Deserialize, Serialize};

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

/// A party summary.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PartySummary {
	/// A universally unique identifier.
	pub party_id: std::string::String,
	/// RFC3339 timestamp.
	pub create_ts: chrono::DateTime<chrono::Utc>,
	/// A union representing the activity of a given party. - `Idle`: The party is not doing anything. For example, the leader is sitting in the game menu or the players are hanging out on the hub. - `MatchmakerFindingLobby`: There is a find request in progress for the lobby. If the find request fails, it will go back to `Idle`. If the find request succeeds, it will go to `MatchmakerLobby`. - `MatchmakerLobby`: The party is in a lobby. This does not mean that all of the party members are in the lobby, see the member-specific states.
	pub activity: PartyActivity,
	/// External links for a party.
	pub external: PartyExternalLinks,
	#[allow(missing_docs)] // documentation missing in model
	pub publicity: PartyPublicity,
	/// Unsigned 32 bit integer.
	pub party_size: i32,
	/// A list of party members.
	pub members: std::vec::Vec<PartyMemberSummary>,
	/// A universally unique identifier.
	pub thread_id: std::string::String,
}

/// A party member summary.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PartyMemberSummary {
	/// An identity handle.
	pub identity: IdentityHandle,
	/// Whether or not this party member is the leader of the given party.
	pub is_leader: bool,
	/// RFC3339 timestamp.
	pub join_ts: chrono::DateTime<chrono::Utc>,
	/// A union representing the current state of a party member. - `Inactive`: The player is not doing anything. For example, the player can be sitting in the game menu or hanging out on the hub. - It's possible for the member to be in an inactive state while the party is in a lobby; this means the player is simply observing/interacting with others in the party and not part of the matchmaking process. - `MatchmakerReady`: This means the member wants a player created for them. - Members can be in the ready state while the party is in an idle state. This means that the player will get a player created for them. - Members can be in the ready state while the party is in a lobby. This means that the player could not join the lobby because it was full or the player left the lobby unintentionally. - `MatchmakerFindingLobby`: A find request is in progress for the member. - `MatchmakerLobby`: The member is in a lobby.
	pub state: PartyMemberState,
}

/// A union representing the current state of a party member. - `Inactive`: The player is not doing anything. For example, the player can be sitting in the game menu or hanging out on the hub. - It's possible for the member to be in an inactive state while the party is in a lobby; this means the player is simply observing/interacting with others in the party and not part of the matchmaking process. - `MatchmakerReady`: This means the member wants a player created for them. - Members can be in the ready state while the party is in an idle state. This means that the player will get a player created for them. - Members can be in the ready state while the party is in a lobby. This means that the player could not join the lobby because it was full or the player left the lobby unintentionally. - `MatchmakerFindingLobby`: A find request is in progress for the member. - `MatchmakerLobby`: The member is in a lobby.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PartyMemberState {
	/// A party member state denoting that the member is inactive.
	Inactive(PartyMemberStateInactive),
	/// A party member state denoting that the member is currently searching for a lobby.
	MatchmakerFindingLobby(PartyMemberStateMatchmakerFindingLobby),
	/// A party member state denoting that the member is in a lobby.
	MatchmakerLobby(PartyMemberStateMatchmakerLobby),
	/// A party member state denoting that the member is currently waiting to start matchmaking.
	MatchmakerReady(PartyMemberStateMatchmakerReady),
}

/// A party member state denoting that the member is in a lobby.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PartyMemberStateMatchmakerLobby {
	/// A universally unique identifier.
	pub player_id: std::string::String,
}

/// A party member state denoting that the member is currently searching for a lobby.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PartyMemberStateMatchmakerFindingLobby {}

/// A party member state denoting that the member is currently waiting to start matchmaking.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PartyMemberStateMatchmakerReady {}

/// A party member state denoting that the member is inactive.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PartyMemberStateInactive {}

/// An identity handle.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct IdentityHandle {
	/// A universally unique identifier.
	pub identity_id: std::string::String,
	/// Represent a resource's readable display name.
	pub display_name: std::string::String,
	/// Identity profile account number (#1234). These are assigned in addition to an identity's display name in order to allow multiple identities to have the same display name while still providing a unique handle. These are unique to each display name; you can have multiple accounts with different display names and the same account number.
	pub account_number: i32,
	/// The URL of this identity's avatar image.
	pub avatar_url: std::string::String,
	/// Information about the identity's current status, party, and active game.
	pub presence: std::option::Option<IdentityPresence>,
	/// A party handle.
	pub party: std::option::Option<PartyHandle>,
	/// Whether or not this identity is registered with a linked account.
	pub is_registered: bool,
	/// External links for an identity.
	pub external: IdentityExternalLinks,
}

/// External links for an identity.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct IdentityExternalLinks {
	/// A link to this identity's profile page.
	pub profile: std::string::String,
	/// A link to the Rivet settings page.
	pub settings: std::option::Option<std::string::String>,
	/// A link to a chat page with the given identity.
	pub chat: std::option::Option<std::string::String>,
}

/// A party handle.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PartyHandle {
	/// A universally unique identifier.
	pub party_id: std::string::String,
	/// RFC3339 timestamp.
	pub create_ts: chrono::DateTime<chrono::Utc>,
	/// A union representing the activity of a given party. - `Idle`: The party is not doing anything. For example, the leader is sitting in the game menu or the players are hanging out on the hub. - `MatchmakerFindingLobby`: There is a find request in progress for the lobby. If the find request fails, it will go back to `Idle`. If the find request succeeds, it will go to `MatchmakerLobby`. - `MatchmakerLobby`: The party is in a lobby. This does not mean that all of the party members are in the lobby, see the member-specific states.
	pub activity: PartyActivity,
	/// External links for a party.
	pub external: PartyExternalLinks,
}

/// External links for a party.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PartyExternalLinks {
	/// A link to the given party's chat thread.
	pub chat: std::string::String,
}

/// A union representing the activity of a given party. - `Idle`: The party is not doing anything. For example, the leader is sitting in the game menu or the players are hanging out on the hub. - `MatchmakerFindingLobby`: There is a find request in progress for the lobby. If the find request fails, it will go back to `Idle`. If the find request succeeds, it will go to `MatchmakerLobby`. - `MatchmakerLobby`: The party is in a lobby. This does not mean that all of the party members are in the lobby, see the member-specific states.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PartyActivity {
	/// A party activity denoting that the party is idle.
	Idle(PartyActivityIdle),
	/// A party activity denoting that the party is currently searching for a lobby.
	MatchmakerFindingLobby(PartyActivityMatchmakerFindingLobby),
	/// A party activity denoting that the party is currently in a lobby.
	MatchmakerLobby(PartyActivityMatchmakerLobby),
}

/// A party activity denoting that the party is currently in a lobby.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PartyActivityMatchmakerLobby {
	/// A party lobby.
	pub lobby: PartyMatchmakerLobby,
	/// A game handle.
	pub game: GameHandle,
}

/// A game handle.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GameHandle {
	/// A universally unique identifier.
	pub game_id: std::string::String,
	/// A human readable short identifier used to references resources. Different than a `rivet.common#Uuid` because this is intended to be human readable. Different than `rivet.common#DisplayName` because this should not include special characters and be short.
	pub name_id: std::string::String,
	/// Represent a resource's readable display name.
	pub display_name: std::string::String,
	/// The URL of this game's logo image.
	pub logo_url: std::option::Option<std::string::String>,
	/// The URL of this game's banner image.
	pub banner_url: std::option::Option<std::string::String>,
}

/// A party lobby.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PartyMatchmakerLobby {
	/// A universally unique identifier.
	pub lobby_id: std::string::String,
}

/// A party activity denoting that the party is currently searching for a lobby.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PartyActivityMatchmakerFindingLobby {
	/// A game handle.
	pub game: GameHandle,
}

/// A party activity denoting that the party is idle.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PartyActivityIdle {}

/// Information about the identity's current status, party, and active game.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct IdentityPresence {
	/// RFC3339 timestamp.
	pub update_ts: chrono::DateTime<chrono::Utc>,
	/// The current status of an identity. This helps players understand if another player is currently playing or has their game in the background.
	pub status: IdentityStatus,
	/// The game an identity is currently participating in.
	pub game_activity: std::option::Option<IdentityGameActivity>,
}

/// The game an identity is currently participating in.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct IdentityGameActivity {
	/// A game handle.
	pub game: GameHandle,
	/// A short activity message about the current game activity.
	pub message: std::string::String,
	/// JSON data seen by anyone.
	pub public_metadata: std::option::Option<serde_json::Value>,
	/// JSON data seen only by the given identity and their mutual followers.
	pub mutual_metadata: std::option::Option<serde_json::Value>,
}

/// The current status of an identity. This helps players understand if another
/// player is currently playing or has their game in the background.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Hash, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum IdentityStatus {
	#[allow(missing_docs)] // documentation missing in model
	Away,
	#[allow(missing_docs)] // documentation missing in model
	Offline,
	#[allow(missing_docs)] // documentation missing in model
	Online,
	/// Unknown contains new variants that have been added since this code was generated.
	Unknown(String),
}
impl std::convert::From<&str> for IdentityStatus {
	fn from(s: &str) -> Self {
		match s {
			"away" => IdentityStatus::Away,
			"offline" => IdentityStatus::Offline,
			"online" => IdentityStatus::Online,
			other => IdentityStatus::Unknown(other.to_owned()),
		}
	}
}
impl std::str::FromStr for IdentityStatus {
	type Err = std::convert::Infallible;

	fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
		Ok(IdentityStatus::from(s))
	}
}
impl IdentityStatus {
	/// Returns the `&str` value of the enum member.
	pub fn as_str(&self) -> &str {
		match self {
			IdentityStatus::Away => "away",
			IdentityStatus::Offline => "offline",
			IdentityStatus::Online => "online",
			IdentityStatus::Unknown(s) => s.as_ref(),
		}
	}
	/// Returns all the `&str` values of the enum members.
	pub fn values() -> &'static [&'static str] {
		&["away", "offline", "online"]
	}
}
impl AsRef<str> for IdentityStatus {
	fn as_ref(&self) -> &str {
		self.as_str()
	}
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PartyPublicity {
	#[allow(missing_docs)] // documentation missing in model
	pub public: PartyPublicityLevel,
	#[allow(missing_docs)] // documentation missing in model
	pub mutual_followers: PartyPublicityLevel,
	#[allow(missing_docs)] // documentation missing in model
	pub groups: PartyPublicityLevel,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Hash, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PartyPublicityLevel {
	#[allow(missing_docs)] // documentation missing in model
	Join,
	#[allow(missing_docs)] // documentation missing in model
	None,
	#[allow(missing_docs)] // documentation missing in model
	View,
	/// Unknown contains new variants that have been added since this code was generated.
	Unknown(String),
}
impl std::convert::From<&str> for PartyPublicityLevel {
	fn from(s: &str) -> Self {
		match s {
			"join" => PartyPublicityLevel::Join,
			"none" => PartyPublicityLevel::None,
			"view" => PartyPublicityLevel::View,
			other => PartyPublicityLevel::Unknown(other.to_owned()),
		}
	}
}
impl std::str::FromStr for PartyPublicityLevel {
	type Err = std::convert::Infallible;

	fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
		Ok(PartyPublicityLevel::from(s))
	}
}
impl PartyPublicityLevel {
	/// Returns the `&str` value of the enum member.
	pub fn as_str(&self) -> &str {
		match self {
			PartyPublicityLevel::Join => "join",
			PartyPublicityLevel::None => "none",
			PartyPublicityLevel::View => "view",
			PartyPublicityLevel::Unknown(s) => s.as_ref(),
		}
	}
	/// Returns all the `&str` values of the enum members.
	pub fn values() -> &'static [&'static str] {
		&["join", "none", "view"]
	}
}
impl AsRef<str> for PartyPublicityLevel {
	fn as_ref(&self) -> &str {
		self.as_str()
	}
}

/// Represents methods of joining a party.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum JoinPartyInvite {
	/// A party invite alias. See `rivet.api.party.common#CreatePartyInviteConfig$alias` and `rivet.api.party#CreatePartyInvite$alias`.
	Alias(std::string::String),
	/// Requires the party publicity to this identity to be `rivet.party#PartyPublicityLevel$JOIN`.
	PartyId(std::string::String),
	/// A party invite token. See `rivet.api.party.common#CreatedInvite$token`.
	Token(std::string::String),
}

/// Output from a created invite.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CreatedInvite {
	/// An alias used to join a given party. This alias must be unique for all invites for your game. Pass this alias to `rivet.api.party.common#CreatedInvite$alias` to consume the invite.
	pub alias: std::option::Option<std::string::String>,
	/// A JSON Web Token. Slightly modified to include a description prefix and use Protobufs of JSON.
	pub token: std::string::String,
}

/// Configuration for creating a party invite.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CreatePartyInviteConfig {
	/// An alias used to join a given party. This alias must be unique for all invites for your game. Pass this alias to `rivet.api.party.common#CreatedInvite$alias` to consume the invite.
	pub alias: std::option::Option<std::string::String>,
}

/// Publicity configuration for creating a party. Null values will default
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CreatePartyPublicityConfig {
	/// Defaults to `rivet.party#PartyPublicityLevel$VIEW`.
	pub public: std::option::Option<PartyPublicityLevel>,
	/// Defaults to `rivet.party#PartyPublicityLevel$JOIN`.
	pub mutual_followers: std::option::Option<PartyPublicityLevel>,
	/// Defaults to `rivet.party#PartyPublicityLevel$VIEW`.
	pub groups: std::option::Option<PartyPublicityLevel>,
}

/// Provided by watchable endpoints used in blocking loops.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct WatchResponse {
	/// Index indicating the version of the data responded. Pas this to `rivet.common#WatchQuery` to block and wait for the next response.
	pub index: std::string::String,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PartyProfile {
	/// A universally unique identifier.
	pub party_id: std::string::String,
	/// RFC3339 timestamp.
	pub create_ts: chrono::DateTime<chrono::Utc>,
	/// A union representing the activity of a given party. - `Idle`: The party is not doing anything. For example, the leader is sitting in the game menu or the players are hanging out on the hub. - `MatchmakerFindingLobby`: There is a find request in progress for the lobby. If the find request fails, it will go back to `Idle`. If the find request succeeds, it will go to `MatchmakerLobby`. - `MatchmakerLobby`: The party is in a lobby. This does not mean that all of the party members are in the lobby, see the member-specific states.
	pub activity: PartyActivity,
	/// External links for a party.
	pub external: PartyExternalLinks,
	#[allow(missing_docs)] // documentation missing in model
	pub publicity: PartyPublicity,
	/// Unsigned 32 bit integer.
	pub party_size: i32,
	/// A list of party members.
	pub members: std::vec::Vec<PartyMemberSummary>,
	/// A universally unique identifier.
	pub thread_id: std::string::String,
	/// A list of party invites.
	pub invites: std::vec::Vec<PartyInvite>,
}

/// A party invite.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PartyInvite {
	/// A universally unique identifier.
	pub invite_id: std::string::String,
	/// RFC3339 timestamp.
	pub create_ts: chrono::DateTime<chrono::Utc>,
	/// A JSON Web Token. Slightly modified to include a description prefix and use Protobufs of JSON.
	pub token: std::string::String,
	/// An alias used to join a given party.
	pub alias: std::option::Option<PartyInviteAlias>,
	/// Extenral links for a party invite.
	pub external: PartyInviteExternalLinks,
}

/// Extenral links for a party invite.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PartyInviteExternalLinks {
	/// The invite link used to join this party from an external site.
	pub invite: std::string::String,
}

/// An alias used to join a given party.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PartyInviteAlias {
	/// A universally unique identifier.
	pub namespace_id: std::string::String,
	/// The alias used to join a given party.
	pub alias: std::string::String,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct MatchmakerSelfReadyRequest {}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct FindMatchmakerLobbyForPartyRequest {
	/// Game modes to match lobbies against.
	pub game_modes: std::vec::Vec<std::string::String>,
	/// Regions to match lobbies against. If not specified, the optimal region will be determined and will attempt to find lobbies in that region.
	pub regions: std::option::Option<std::vec::Vec<std::string::String>>,
	/// Prevents a new lobby from being created when finding a lobby. If no lobby is found, `MATCHMAKER_LOBBY_NOT_FOUND` will be returned.
	pub prevent_auto_create_lobby: std::option::Option<bool>,
	/// Methods to verify a captcha.
	pub captcha: std::option::Option<CaptchaConfig>,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct JoinMatchmakerLobbyForPartyRequest {
	/// A universally unique identifier.
	pub lobby_id: std::string::String,
	/// Methods to verify a captcha.
	pub captcha: std::option::Option<CaptchaConfig>,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SetSelfInactiveRequest {}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SetPartyToIdleRequest {}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SendJoinRequestRequest {}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetPartyFromInviteRequest {}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RevokePartyInviteRequest {}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct KickMemberRequest {}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TransferPartyOwnershipRequest {}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SetPartyPublicityRequest {
	/// Defaults to `rivet.party#PartyPublicityLevel$VIEW`.
	pub public: std::option::Option<PartyPublicityLevel>,
	/// Defaults to `rivet.party#PartyPublicityLevel$JOIN`.
	pub mutual_followers: std::option::Option<PartyPublicityLevel>,
	/// Defaults to `rivet.party#PartyPublicityLevel$VIEW`.
	pub groups: std::option::Option<PartyPublicityLevel>,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct LeavePartyRequest {}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct JoinPartyRequest {
	/// Represents methods of joining a party.
	pub invite: JoinPartyInvite,
	/// Whether or not to automatically join the game lobby if a party is currently in game.
	pub matchmaker_auto_join_lobby: std::option::Option<bool>,
	/// If the player is currently in the lobby, pass the token from `rivet.matchmaker#MatchmakerLobbyJoinInfoPlayer$token`. This will prevent issuing a new player token.
	pub matchmaker_current_player_token: std::option::Option<std::string::String>,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CreatePartyInviteRequest {
	/// An alias used to join a given party.
	pub alias: std::option::Option<std::string::String>,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CreatePartyRequest {
	/// How many members can join the party. If using this party with the matchmaker, this number should be less than or equal to your party player limit. Super large parties may not be able to fit insite a lobby and be unable to join the game.
	pub party_size: i32,
	/// Publicity configuration for creating a party. Null values will default
	pub publicity: std::option::Option<CreatePartyPublicityConfig>,
	#[allow(missing_docs)] // documentation missing in model
	pub invites: std::option::Option<std::vec::Vec<CreatePartyInviteConfig>>,
	/// If the player is currently in the lobby, pass the token from `rivet.matchmaker#MatchmakerLobbyJoinInfoPlayer$token`. This will prevent issuing a new player token and automatically set the party state to the player's current lobby.
	pub matchmaker_current_player_token: std::option::Option<std::string::String>,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetPartySelfProfileRequest {}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetPartyProfileRequest {}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetPartySelfSummaryRequest {}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetPartySummaryRequest {}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct MatchmakerSelfReadyResponse {}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct FindMatchmakerLobbyForPartyResponse {}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct JoinMatchmakerLobbyForPartyResponse {}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SetSelfInactiveResponse {}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SetPartyToIdleResponse {}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SendJoinRequestResponse {}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetPartyFromInviteResponse {
	/// A party summary.
	pub party: PartySummary,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RevokePartyInviteResponse {}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct KickMemberResponse {}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TransferPartyOwnershipResponse {}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SetPartyPublicityResponse {}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct LeavePartyResponse {}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct JoinPartyResponse {
	/// A universally unique identifier.
	pub party_id: std::string::String,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CreatePartyInviteResponse {
	/// Output from a created invite.
	pub invite: CreatedInvite,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CreatePartyResponse {
	/// A universally unique identifier.
	pub party_id: std::string::String,
	#[allow(missing_docs)] // documentation missing in model
	pub invites: std::vec::Vec<CreatedInvite>,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetPartySelfProfileResponse {
	#[allow(missing_docs)] // documentation missing in model
	pub party: std::option::Option<PartyProfile>,
	/// Provided by watchable endpoints used in blocking loops.
	pub watch: WatchResponse,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetPartyProfileResponse {
	#[allow(missing_docs)] // documentation missing in model
	pub party: PartyProfile,
	/// Provided by watchable endpoints used in blocking loops.
	pub watch: WatchResponse,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetPartySelfSummaryResponse {
	/// A party summary.
	pub party: std::option::Option<PartySummary>,
	/// Provided by watchable endpoints used in blocking loops.
	pub watch: WatchResponse,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetPartySummaryResponse {
	/// A party summary.
	pub party: PartySummary,
	/// Provided by watchable endpoints used in blocking loops.
	pub watch: WatchResponse,
}

