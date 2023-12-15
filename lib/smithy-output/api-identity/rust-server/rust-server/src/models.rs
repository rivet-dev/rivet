#[allow(unused_imports)]
use chrono;
#[allow(unused_imports)]
use serde::{Deserialize, Serialize};

/// Provided by watchable endpoints used in blocking loops.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct WatchResponse {
	/// Index indicating the version of the data responded. Pas this to `rivet.common#WatchQuery` to block and wait for the next response.
	pub index: std::string::String,
}

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

/// A group summary.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GroupSummary {
	/// A universally unique identifier.
	pub group_id: std::string::String,
	/// Represent a resource's readable display name.
	pub display_name: std::string::String,
	/// The URL of this group's avatar image.
	pub avatar_url: std::option::Option<std::string::String>,
	/// External links for this group.
	pub external: GroupExternalLinks,
	/// Whether or not this group is a developer.
	pub is_developer: bool,
	/// Detailed information about a profile.
	pub bio: std::string::String,
	/// Whether or not the current identity is a member of this group.
	pub is_current_identity_member: bool,
	/// The current publicity value for the given group.
	pub publicity: GroupPublicity,
	/// Unsigned 32 bit integer.
	pub member_count: i32,
	/// A universally unique identifier.
	pub owner_identity_id: std::string::String,
}

/// The current publicity value for the given group.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Hash, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GroupPublicity {
	#[allow(missing_docs)] // documentation missing in model
	Closed,
	#[allow(missing_docs)] // documentation missing in model
	Open,
	/// Unknown contains new variants that have been added since this code was generated.
	Unknown(String),
}
impl std::convert::From<&str> for GroupPublicity {
	fn from(s: &str) -> Self {
		match s {
			"closed" => GroupPublicity::Closed,
			"open" => GroupPublicity::Open,
			other => GroupPublicity::Unknown(other.to_owned()),
		}
	}
}
impl std::str::FromStr for GroupPublicity {
	type Err = std::convert::Infallible;

	fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
		Ok(GroupPublicity::from(s))
	}
}
impl GroupPublicity {
	/// Returns the `&str` value of the enum member.
	pub fn as_str(&self) -> &str {
		match self {
			GroupPublicity::Closed => "closed",
			GroupPublicity::Open => "open",
			GroupPublicity::Unknown(s) => s.as_ref(),
		}
	}
	/// Returns all the `&str` values of the enum members.
	pub fn values() -> &'static [&'static str] {
		&["closed", "open"]
	}
}
impl AsRef<str> for GroupPublicity {
	fn as_ref(&self) -> &str {
		self.as_str()
	}
}

/// External links for this group.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GroupExternalLinks {
	/// A link to this group's profile page.
	pub profile: std::string::String,
	/// A link to this group's chat page.
	pub chat: std::string::String,
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

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GameSummary {
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
	/// The URL to this game's website.
	pub url: std::string::String,
	/// A group handle.
	pub developer: GroupHandle,
	/// A list of game tags.
	pub tags: std::vec::Vec<std::string::String>,
}

/// A group handle.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GroupHandle {
	/// A universally unique identifier.
	pub group_id: std::string::String,
	/// Represent a resource's readable display name.
	pub display_name: std::string::String,
	/// The URL of this group's avatar image.
	pub avatar_url: std::option::Option<std::string::String>,
	/// External links for this group.
	pub external: GroupExternalLinks,
	/// Whether or not this group is a developer group.
	pub is_developer: std::option::Option<bool>,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetGameLinkNewIdentity {
	/// See `rivet.api.identity#SetupIdentityOutput$identity_token`.
	pub identity_token: std::string::String,
	/// See `rivet.api.identity#SetupIdentityOutput$identity_token_expire_ts`.
	pub identity_token_expire_ts: chrono::DateTime<chrono::Utc>,
	/// See `rivet.api.identity#SetupIdentityOutput$identity`.
	pub identity: IdentityProfile,
}

/// An identity profile.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct IdentityProfile {
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
	/// A party summary.
	pub party: std::option::Option<PartySummary>,
	/// Whether or not this identity is registered with a linked account.
	pub is_registered: bool,
	/// External links for an identity.
	pub external: IdentityExternalLinks,
	/// Whether or not this identity is an admin.
	pub is_admin: bool,
	/// Whether or not this game user has been linked through the Rivet dashboard.
	pub is_game_linked: std::option::Option<bool>,
	/// The state of the given identity's developer status.
	pub dev_state: std::option::Option<IdentityDevState>,
	/// Unsigned 64 bit integer.
	pub follower_count: i64,
	/// Unsigned 64 bit integer.
	pub following_count: i64,
	/// Whether or not the requestee's identity is following this identity.
	pub following: bool,
	/// Whether or not this identity following the requestee's identity.
	pub is_following_me: bool,
	/// Whether or not this identity is both following and is followed by the requestee's identity.
	pub is_mutual_following: bool,
	/// RFC3339 timestamp.
	pub join_ts: chrono::DateTime<chrono::Utc>,
	/// Detailed information about a profile.
	pub bio: std::string::String,
	/// A list of an identity's linked accounts.
	pub linked_accounts: std::vec::Vec<IdentityLinkedAccount>,
	/// A list of groups that the given identity is in.
	pub groups: std::vec::Vec<IdentityGroup>,
	/// A list of game statistic summaries.
	pub games: std::vec::Vec<GameStatSummary>,
	/// Whether or not this identity is awaiting account deletion. Only visible to when the requestee is this identity.
	pub awaiting_deletion: std::option::Option<bool>,
}

/// A game statistic summary.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GameStatSummary {
	/// A game handle.
	pub game: GameHandle,
	/// A list of game statistics.
	pub stats: std::vec::Vec<GameStat>,
}

/// A game statistic.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GameStat {
	/// A game statistic config.
	pub config: GameStatConfig,
	/// A single overall value of the given statistic.
	pub overall_value: f32,
}

/// A game statistic config.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GameStatConfig {
	/// A universally unique identifier.
	pub record_id: std::string::String,
	/// A universally unique identifier.
	pub icon_id: std::string::String,
	/// A value denoting the format method of a game statistic.
	pub format: GameStatFormatMethod,
	/// A value denoting the aggregation method of a game statistic.
	pub aggregation: GameStatAggregationMethod,
	/// A value denoting the sorting method of a game statistic.
	pub sorting: GameStatSortingMethod,
	/// Unsigned 32 bit integer.
	pub priority: i32,
	/// Represent a resource's readable display name.
	pub display_name: std::string::String,
	/// A string appended to the end of a singular game statistic's value. Example: 1 **dollar**.
	pub postfix_singular: std::option::Option<std::string::String>,
	/// A string appended to the end of a game statistic's value that is not exactly 1. Example: 45 **dollars**.
	pub postfix_plural: std::option::Option<std::string::String>,
	/// A string appended to the beginning of a singular game statistic's value. Example: **value** 1.
	pub prefix_singular: std::option::Option<std::string::String>,
	/// A string prepended to the beginning of a game statistic's value that is not exactly 1. Example: **values** 45.
	pub prefix_plural: std::option::Option<std::string::String>,
}

/// A value denoting the sorting method of a game statistic.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Hash, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GameStatSortingMethod {
	/// Ascending sorting.
	Asc,
	/// Descending sorting.
	Desc,
	/// Unknown contains new variants that have been added since this code was generated.
	Unknown(String),
}
impl std::convert::From<&str> for GameStatSortingMethod {
	fn from(s: &str) -> Self {
		match s {
			"asc" => GameStatSortingMethod::Asc,
			"desc" => GameStatSortingMethod::Desc,
			other => GameStatSortingMethod::Unknown(other.to_owned()),
		}
	}
}
impl std::str::FromStr for GameStatSortingMethod {
	type Err = std::convert::Infallible;

	fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
		Ok(GameStatSortingMethod::from(s))
	}
}
impl GameStatSortingMethod {
	/// Returns the `&str` value of the enum member.
	pub fn as_str(&self) -> &str {
		match self {
			GameStatSortingMethod::Asc => "asc",
			GameStatSortingMethod::Desc => "desc",
			GameStatSortingMethod::Unknown(s) => s.as_ref(),
		}
	}
	/// Returns all the `&str` values of the enum members.
	pub fn values() -> &'static [&'static str] {
		&["asc", "desc"]
	}
}
impl AsRef<str> for GameStatSortingMethod {
	fn as_ref(&self) -> &str {
		self.as_str()
	}
}

/// A value denoting the aggregation method of a game statistic.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Hash, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GameStatAggregationMethod {
	/// Average aggergation.
	Average,
	/// Maximum value aggregation.
	Max,
	/// Minimum value aggregation.
	Min,
	/// Summation aggregation.
	Sum,
	/// Unknown contains new variants that have been added since this code was generated.
	Unknown(String),
}
impl std::convert::From<&str> for GameStatAggregationMethod {
	fn from(s: &str) -> Self {
		match s {
			"average" => GameStatAggregationMethod::Average,
			"max" => GameStatAggregationMethod::Max,
			"min" => GameStatAggregationMethod::Min,
			"sum" => GameStatAggregationMethod::Sum,
			other => GameStatAggregationMethod::Unknown(other.to_owned()),
		}
	}
}
impl std::str::FromStr for GameStatAggregationMethod {
	type Err = std::convert::Infallible;

	fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
		Ok(GameStatAggregationMethod::from(s))
	}
}
impl GameStatAggregationMethod {
	/// Returns the `&str` value of the enum member.
	pub fn as_str(&self) -> &str {
		match self {
			GameStatAggregationMethod::Average => "average",
			GameStatAggregationMethod::Max => "max",
			GameStatAggregationMethod::Min => "min",
			GameStatAggregationMethod::Sum => "sum",
			GameStatAggregationMethod::Unknown(s) => s.as_ref(),
		}
	}
	/// Returns all the `&str` values of the enum members.
	pub fn values() -> &'static [&'static str] {
		&["average", "max", "min", "sum"]
	}
}
impl AsRef<str> for GameStatAggregationMethod {
	fn as_ref(&self) -> &str {
		self.as_str()
	}
}

/// A value denoting the format method of a game statistic.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Hash, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GameStatFormatMethod {
	/// A duration with hundredth-second precision (1d 2h 45m 21.46s).
	DurationHundredthSecond,
	/// A duration with minute precision (1d 2h 45m).
	DurationMinute,
	/// A duration with second precision (1d 2h 45m 21s).
	DuractionSecond,
	/// A float with 1 decimal (1,234.5).
	Float1,
	/// A float with 2 decimals (1,234.56).
	Float2,
	/// A float with 3 decimals (1,234.567).
	Float3,
	/// An integer with no decimals (1,234).
	Integer,
	/// Unknown contains new variants that have been added since this code was generated.
	Unknown(String),
}
impl std::convert::From<&str> for GameStatFormatMethod {
	fn from(s: &str) -> Self {
		match s {
			"duration_hundredth_second" => GameStatFormatMethod::DurationHundredthSecond,
			"duration_minute" => GameStatFormatMethod::DurationMinute,
			"duration_second" => GameStatFormatMethod::DuractionSecond,
			"float_1" => GameStatFormatMethod::Float1,
			"float_2" => GameStatFormatMethod::Float2,
			"float_3" => GameStatFormatMethod::Float3,
			"integer" => GameStatFormatMethod::Integer,
			other => GameStatFormatMethod::Unknown(other.to_owned()),
		}
	}
}
impl std::str::FromStr for GameStatFormatMethod {
	type Err = std::convert::Infallible;

	fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
		Ok(GameStatFormatMethod::from(s))
	}
}
impl GameStatFormatMethod {
	/// Returns the `&str` value of the enum member.
	pub fn as_str(&self) -> &str {
		match self {
			GameStatFormatMethod::DurationHundredthSecond => "duration_hundredth_second",
			GameStatFormatMethod::DurationMinute => "duration_minute",
			GameStatFormatMethod::DuractionSecond => "duration_second",
			GameStatFormatMethod::Float1 => "float_1",
			GameStatFormatMethod::Float2 => "float_2",
			GameStatFormatMethod::Float3 => "float_3",
			GameStatFormatMethod::Integer => "integer",
			GameStatFormatMethod::Unknown(s) => s.as_ref(),
		}
	}
	/// Returns all the `&str` values of the enum members.
	pub fn values() -> &'static [&'static str] {
		&[
			"duration_hundredth_second",
			"duration_minute",
			"duration_second",
			"float_1",
			"float_2",
			"float_3",
			"integer",
		]
	}
}
impl AsRef<str> for GameStatFormatMethod {
	fn as_ref(&self) -> &str {
		self.as_str()
	}
}

/// A group that the given identity.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct IdentityGroup {
	/// A group handle.
	pub group: GroupHandle,
}

/// A union representing an identity's linked accounts.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum IdentityLinkedAccount {
	/// An identity's linked email.
	Email(IdentityEmailLinkedAccount),
}

/// An identity's linked email.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct IdentityEmailLinkedAccount {
	/// A valid email address.
	pub email: std::string::String,
}

/// The state of the given identity's developer status.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Hash, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum IdentityDevState {
	#[allow(missing_docs)] // documentation missing in model
	Accepted,
	#[allow(missing_docs)] // documentation missing in model
	Inactive,
	#[allow(missing_docs)] // documentation missing in model
	Pending,
	/// Unknown contains new variants that have been added since this code was generated.
	Unknown(String),
}
impl std::convert::From<&str> for IdentityDevState {
	fn from(s: &str) -> Self {
		match s {
			"accepted" => IdentityDevState::Accepted,
			"inactive" => IdentityDevState::Inactive,
			"pending" => IdentityDevState::Pending,
			other => IdentityDevState::Unknown(other.to_owned()),
		}
	}
}
impl std::str::FromStr for IdentityDevState {
	type Err = std::convert::Infallible;

	fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
		Ok(IdentityDevState::from(s))
	}
}
impl IdentityDevState {
	/// Returns the `&str` value of the enum member.
	pub fn as_str(&self) -> &str {
		match self {
			IdentityDevState::Accepted => "accepted",
			IdentityDevState::Inactive => "inactive",
			IdentityDevState::Pending => "pending",
			IdentityDevState::Unknown(s) => s.as_ref(),
		}
	}
	/// Returns all the `&str` values of the enum members.
	pub fn values() -> &'static [&'static str] {
		&["accepted", "inactive", "pending"]
	}
}
impl AsRef<str> for IdentityDevState {
	fn as_ref(&self) -> &str {
		self.as_str()
	}
}

/// The link status between an identity and a game user.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Hash, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GameLinkStatus {
	#[allow(missing_docs)] // documentation missing in model
	Cancelled,
	#[allow(missing_docs)] // documentation missing in model
	Complete,
	#[allow(missing_docs)] // documentation missing in model
	Incomplete,
	/// Unknown contains new variants that have been added since this code was generated.
	Unknown(String),
}
impl std::convert::From<&str> for GameLinkStatus {
	fn from(s: &str) -> Self {
		match s {
			"cancelled" => GameLinkStatus::Cancelled,
			"complete" => GameLinkStatus::Complete,
			"incomplete" => GameLinkStatus::Incomplete,
			other => GameLinkStatus::Unknown(other.to_owned()),
		}
	}
}
impl std::str::FromStr for GameLinkStatus {
	type Err = std::convert::Infallible;

	fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
		Ok(GameLinkStatus::from(s))
	}
}
impl GameLinkStatus {
	/// Returns the `&str` value of the enum member.
	pub fn as_str(&self) -> &str {
		match self {
			GameLinkStatus::Cancelled => "cancelled",
			GameLinkStatus::Complete => "complete",
			GameLinkStatus::Incomplete => "incomplete",
			GameLinkStatus::Unknown(s) => s.as_ref(),
		}
	}
	/// Returns all the `&str` values of the enum members.
	pub fn values() -> &'static [&'static str] {
		&["cancelled", "complete", "incomplete"]
	}
}
impl AsRef<str> for GameLinkStatus {
	fn as_ref(&self) -> &str {
		self.as_str()
	}
}

/// An event relevant to the current identity.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GlobalEvent {
	/// RFC3339 timestamp.
	pub ts: chrono::DateTime<chrono::Utc>,
	/// Kind of event that occurred.
	pub kind: GlobalEventKind,
	/// Notifications represent information that should be presented to the user immediately. At the moment, only chat message events have associated notifications. # Display Notifications should be displayed in an unobtrusive manner throughout the entire game. Notifications should disappear after a few seconds if not interacted with. # Interactions If your platform supports it, notifications should be able to be clicked or tapped in order to open the relevant context for the event. For a simple implementation of notification interactions, open `url` in a web browser to present the relevant context. For example, a chat message notification will open the thread the chat message was sent in. For advanced implementations that implement a custom chat UI, use `rivet.api.identity.common#GlobalEvent$kind` to determine what action to take when the notification is interacted with. For example, if the global event kind is `rivet.api.identity.common#GlobalEventChatMessage`, then open the chat UI for the given thread.
	pub notification: std::option::Option<GlobalEventNotification>,
}

/// Notifications represent information that should be presented to the user immediately. At the moment, only chat message events have associated notifications. # Display Notifications should be displayed in an unobtrusive manner throughout the entire game. Notifications should disappear after a few seconds if not interacted with. # Interactions If your platform supports it, notifications should be able to be clicked or tapped in order to open the relevant context for the event. For a simple implementation of notification interactions, open `url` in a web browser to present the relevant context. For example, a chat message notification will open the thread the chat message was sent in. For advanced implementations that implement a custom chat UI, use `rivet.api.identity.common#GlobalEvent$kind` to determine what action to take when the notification is interacted with. For example, if the global event kind is `rivet.api.identity.common#GlobalEventChatMessage`, then open the chat UI for the given thread.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GlobalEventNotification {
	#[allow(missing_docs)] // documentation missing in model
	pub title: std::string::String,
	#[allow(missing_docs)] // documentation missing in model
	pub description: std::string::String,
	/// URL to an image thumbnail that should be shown for this notification.
	pub thumbnail_url: std::string::String,
	/// Rivet Hub URL that holds the relevant context for this notification.
	pub url: std::string::String,
}

/// Kind of event that occurred.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GlobalEventKind {
	/// `rivet.api.identity.common#GlobalEventKind` variant for chat messages. Received any time a message is sent to a chat the identity is in.
	ChatMessage(GlobalEventChatMessage),
	/// `rivet.api.identity.common#GlobalEventKind` variant for chat reads. Received any time the last read timestamp is set. Used to update the status of unread indicators on chats.
	ChatRead(GlobalEventChatRead),
	/// `rivet.api.identity.common#GlobalEventKind` variant for a chat thread being removed. Received any time the current identity is no longer able to access the given thread. This can happen if
	ChatThreadRemove(GlobalEventChatThreadRemove),
	/// `rivet.api.identity.common#GlobalEventKind` variant for identity updates. Received any time identity details are changed OR the identity switches.
	IdentityUpdate(GlobalEventIdentityUpdate),
	/// `rivet.api.identity.common#GlobalEventKind` variant for party updates. Received when the identity should be instructed to join a lobby. This needs to be implemented in conjunction with parties in order to force clients to join the same lobby as the party they're in.
	MatchmakerLobbyJoin(GlobalEventMatchmakerLobbyJoin),
	/// `rivet.api.identity.common#GlobalEventKind` variant for party updates. Received any time the identity joins a party, a party is updated, or when the identity leaves a party.
	PartyUpdate(GlobalEventPartyUpdate),
}

/// `rivet.api.identity.common#GlobalEventKind` variant for a chat thread being removed. Received any time the current identity is no longer able to access the given thread. This can happen if
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GlobalEventChatThreadRemove {
	/// A universally unique identifier.
	pub thread_id: std::string::String,
}

/// `rivet.api.identity.common#GlobalEventKind` variant for party updates. Received when the identity should be instructed to join a lobby. This needs to be implemented in conjunction with parties in order to force clients to join the same lobby as the party they're in.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GlobalEventMatchmakerLobbyJoin {
	/// A matchmaker lobby.
	pub lobby: MatchmakerLobbyJoinInfo,
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

/// `rivet.api.identity.common#GlobalEventKind` variant for identity updates. Received any time identity details are changed OR the identity switches.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GlobalEventIdentityUpdate {
	/// An identity profile.
	pub identity: IdentityProfile,
}

/// `rivet.api.identity.common#GlobalEventKind` variant for party updates. Received any time the identity joins a party, a party is updated, or when the identity leaves a party.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GlobalEventPartyUpdate {
	/// If null, the identity left the party.
	pub party: std::option::Option<PartySummary>,
}

/// `rivet.api.identity.common#GlobalEventKind` variant for chat reads. Received any time the last read timestamp is set. Used to update the status of unread indicators on chats.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GlobalEventChatRead {
	/// A universally unique identifier.
	pub thread_id: std::string::String,
	/// RFC3339 timestamp.
	pub read_ts: chrono::DateTime<chrono::Utc>,
}

/// `rivet.api.identity.common#GlobalEventKind` variant for chat messages. Received any time a message is sent to a chat the identity is in.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GlobalEventChatMessage {
	/// A chat thread.
	pub thread: ChatThread,
}

/// A chat thread.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ChatThread {
	/// A universally unique identifier.
	pub thread_id: std::string::String,
	/// RFC3339 timestamp.
	pub create_ts: chrono::DateTime<chrono::Utc>,
	/// Represents the topic of the given chat thread.
	pub topic: ChatTopic,
	/// A chat message.
	pub tail_message: std::option::Option<ChatMessage>,
	/// RFC3339 timestamp.
	pub last_read_ts: chrono::DateTime<chrono::Utc>,
	/// Unsigned 64 bit integer.
	pub unread_count: i64,
	/// External links for a chat thread.
	pub external: ChatThreadExternalLinks,
}

/// External links for a chat thread.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ChatThreadExternalLinks {
	/// A link to opening the chat thread.
	pub chat: std::string::String,
}

/// A chat message.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ChatMessage {
	/// A universally unique identifier.
	pub chat_message_id: std::string::String,
	/// A universally unique identifier.
	pub thread_id: std::string::String,
	/// RFC3339 timestamp.
	pub send_ts: chrono::DateTime<chrono::Utc>,
	/// Represents types of chat message bodies.
	pub body: ChatMessageBody,
}

/// Represents types of chat message bodies.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ChatMessageBody {
	/// `rivet.chat#ChatMessageBody` variant for indicating a new chat was created.
	ChatCreate(ChatMessageBodyChatCreate),
	/// `rivet.chat#ChatMessageBody` variant for deleted messages.
	Deleted(ChatMessageBodyDeleted),
	/// `rivet.chat#ChatMessageBody` variant for indicating an identity joined the group.
	GroupJoin(ChatMessageBodyGroupJoin),
	/// `rivet.chat#ChatMessageBody` variant for indicating an identity left the group.
	GroupLeave(ChatMessageBodyGroupLeave),
	/// `rivet.chat#ChatMessageBody` variant for indicating an identity has been kicked from the group.
	GroupMemberKick(ChatMessageBodyGroupMemberKick),
	/// `rivet.chat#ChatMessageBody` variant for indicating an identity followed the identity.
	IdentityFollow(ChatMessageBodyIdentityFollow),
	/// `rivet.chat#ChatMessageBody` variant for indicating a change in the party's current activity.
	PartyActivityChange(ChatMessageBodyPartyActivityChange),
	/// `rivet.chat#ChatMessageBody` variant holding an a party invitation.
	PartyInvite(ChatMessageBodyPartyInvite),
	/// `rivet.chat#ChatMessageBody` variant for indicating an identity joined the party.
	PartyJoin(ChatMessageBodyPartyJoin),
	/// `rivet.chat#ChatMessageBody` variant for indicating an identity requesting to join your party.
	PartyJoinRequest(ChatMessageBodyPartyJoinRequest),
	/// `rivet.chat#ChatMessageBody` variant for indicating an identity left the party.
	PartyLeave(ChatMessageBodyPartyLeave),
	/// `rivet.chat#ChatMessageBody` variant for text messages. Sent by other identities using the chat interface.
	Text(ChatMessageBodyText),
}

/// `rivet.chat#ChatMessageBody` variant for indicating a change in the party's current activity.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ChatMessageBodyPartyActivityChange {
	/// A union representing the activity of a given party. - `Idle`: The party is not doing anything. For example, the leader is sitting in the game menu or the players are hanging out on the hub. - `MatchmakerFindingLobby`: There is a find request in progress for the lobby. If the find request fails, it will go back to `Idle`. If the find request succeeds, it will go to `MatchmakerLobby`. - `MatchmakerLobby`: The party is in a lobby. This does not mean that all of the party members are in the lobby, see the member-specific states.
	pub activity: PartyActivity,
}

/// `rivet.chat#ChatMessageBody` variant for indicating an identity left the party.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ChatMessageBodyPartyLeave {
	/// An identity handle.
	pub identity: IdentityHandle,
}

/// `rivet.chat#ChatMessageBody` variant for indicating an identity joined the party.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ChatMessageBodyPartyJoin {
	/// An identity handle.
	pub identity: IdentityHandle,
}

/// `rivet.chat#ChatMessageBody` variant for indicating an identity requesting to join your party.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ChatMessageBodyPartyJoinRequest {
	/// An identity handle.
	pub sender: IdentityHandle,
}

/// `rivet.chat#ChatMessageBody` variant holding an a party invitation.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ChatMessageBodyPartyInvite {
	/// An identity handle.
	pub sender: IdentityHandle,
	/// A party handle.
	pub party: std::option::Option<PartyHandle>,
	/// Pass to `rivet.api.party#GetPartyFromInvite$token` to view more information about the party. Pass to `rivet.api.party.common#JoinPartyInvite$token` to join the party.
	pub invite_token: std::option::Option<std::string::String>,
}

/// `rivet.chat#ChatMessageBody` variant for indicating an identity has been kicked from the group.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ChatMessageBodyGroupMemberKick {
	/// An identity handle.
	pub identity: IdentityHandle,
}

/// `rivet.chat#ChatMessageBody` variant for indicating an identity left the group.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ChatMessageBodyGroupLeave {
	/// An identity handle.
	pub identity: IdentityHandle,
}

/// `rivet.chat#ChatMessageBody` variant for indicating an identity joined the group.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ChatMessageBodyGroupJoin {
	/// An identity handle.
	pub identity: IdentityHandle,
}

/// `rivet.chat#ChatMessageBody` variant for indicating an identity followed the identity.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ChatMessageBodyIdentityFollow {}

/// `rivet.chat#ChatMessageBody` variant for deleted messages.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ChatMessageBodyDeleted {
	/// An identity handle.
	pub sender: IdentityHandle,
}

/// `rivet.chat#ChatMessageBody` variant for indicating a new chat was created.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ChatMessageBodyChatCreate {}

/// `rivet.chat#ChatMessageBody` variant for text messages. Sent by other identities using the chat interface.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ChatMessageBodyText {
	/// An identity handle.
	pub sender: IdentityHandle,
	/// The text in the message.
	pub body: std::string::String,
}

/// Represents the topic of the given chat thread.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ChatTopic {
	/// `rivet.chat#ChatTopic` variant for direct (identity to identity) chats.
	Direct(ChatTopicDirect),
	/// `rivet.chat#ChatTopic` variant for groups.
	Group(ChatTopicGroup),
	/// `rivet.chat#ChatTopic` variant for parties.
	Party(ChatTopicParty),
}

/// `rivet.chat#ChatTopic` variant for direct (identity to identity) chats.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ChatTopicDirect {
	/// An identity handle.
	pub identity_a: IdentityHandle,
	/// An identity handle.
	pub identity_b: IdentityHandle,
}

/// `rivet.chat#ChatTopic` variant for parties.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ChatTopicParty {
	/// A party handle.
	pub party: PartyHandle,
}

/// `rivet.chat#ChatTopic` variant for groups.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ChatTopicGroup {
	/// A group handle.
	pub group: GroupHandle,
}

/// An identity summary.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct IdentitySummary {
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
	/// Whether or not the requestee's identity is following this identity.
	pub following: bool,
	/// Whether or not this identity following the requestee's identity.
	pub is_following_me: bool,
	/// Whether or not this identity is both following and is followed by the requestee's identity.
	pub is_mutual_following: bool,
}

/// A presigned request used to upload files. Upload your file to the given URL via a PUT request.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct UploadPresignedRequest {
	/// The name of the file to upload. This is the same as the one given in the upload prepare file.
	pub path: std::string::String,
	/// The URL of the presigned request for which to upload your file to.
	pub url: std::string::String,
}

/// Information about the identity's current game. This is information that all other identities can see about what the current identity is doing.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct UpdateIdentityGameActivity {
	/// A short message about the current game activity.
	pub message: std::string::String,
	/// JSON data seen by anyone.
	pub public_metadata: std::option::Option<serde_json::Value>,
	/// JSON data seen only by the given identity and their mutual followers.
	pub mutual_metadata: std::option::Option<serde_json::Value>,
}

/// An error given by failed content validation.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ValidationError {
	/// A list of strings denoting the origin of a validation error.
	pub path: std::vec::Vec<std::string::String>,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ListActivitiesRequest {}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CancelGameLinkRequest {
	/// A JSON Web Token. Slightly modified to include a description prefix and use Protobufs of JSON.
	pub identity_link_token: std::string::String,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CompleteGameLinkRequest {
	/// A JSON Web Token. Slightly modified to include a description prefix and use Protobufs of JSON.
	pub identity_link_token: std::string::String,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetGameLinkRequest {}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PrepareGameLinkRequest {}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct WatchEventsRequest {}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct UnmarkDeletionRequest {}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct MarkDeletionRequest {}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ListMutualFriendsRequest {}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RecentFollowerIgnoreRequest {}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ListRecentFollowersRequest {}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetIdentitySummariesRequest {}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetIdentityHandlesRequest {}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ReportIdentityRequest {
	#[allow(missing_docs)] // documentation missing in model
	pub reason: std::option::Option<std::string::String>,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ListFriendsRequest {}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ListFollowingRequest {}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ListFollowersRequest {}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SignupForBetaRequest {
	#[allow(missing_docs)] // documentation missing in model
	pub name: std::string::String,
	#[allow(missing_docs)] // documentation missing in model
	pub company_name: std::option::Option<std::string::String>,
	#[allow(missing_docs)] // documentation missing in model
	pub company_size: std::string::String,
	#[allow(missing_docs)] // documentation missing in model
	pub preferred_tools: std::string::String,
	#[allow(missing_docs)] // documentation missing in model
	pub goals: std::string::String,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CompleteIdentityAvatarUploadRequest {}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PrepareIdentityAvatarUploadRequest {
	/// The path/filename of the identity avatar.
	pub path: std::string::String,
	/// The MIME type of the identity avatar.
	pub mime: std::option::Option<std::string::String>,
	/// Unsigned 64 bit integer.
	pub content_length: i64,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct UnfollowIdentityRequest {}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct FollowIdentityRequest {}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct UpdateIdentityStatusRequest {
	/// The current status of an identity. This helps players understand if another player is currently playing or has their game in the background.
	pub status: IdentityStatus,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RemoveIdentityGameActivityRequest {}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SetIdentityGameActivityRequest {
	/// Information about the identity's current game. This is information that all other identities can see about what the current identity is doing.
	pub game_activity: UpdateIdentityGameActivity,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SearchIdentitiesRequest {}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ValidateIdentityProfileRequest {
	/// Represent a resource's readable display name.
	pub display_name: std::option::Option<std::string::String>,
	/// Identity profile account number (#1234). These are assigned in addition to an identity's display name in order to allow multiple identities to have the same display name while still providing a unique handle. These are unique to each display name; you can have multiple accounts with different display names and the same account number.
	pub account_number: std::option::Option<i32>,
	/// Detailed information about a profile.
	pub bio: std::option::Option<std::string::String>,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct UpdateIdentityProfileRequest {
	/// Represent a resource's readable display name.
	pub display_name: std::option::Option<std::string::String>,
	/// Identity profile account number (#1234). These are assigned in addition to an identity's display name in order to allow multiple identities to have the same display name while still providing a unique handle. These are unique to each display name; you can have multiple accounts with different display names and the same account number.
	pub account_number: std::option::Option<i32>,
	/// Detailed information about a profile.
	pub bio: std::option::Option<std::string::String>,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetIdentitySelfProfileRequest {}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetIdentityProfileRequest {}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SetupIdentityRequest {
	/// Token returned from previous call to `rivet.api.identity#SetupIdentity`. If this token is invalid, a new identity will be returned.
	pub existing_identity_token: std::option::Option<std::string::String>,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ListActivitiesResponse {
	/// A list of identity handles.
	pub identities: std::vec::Vec<IdentityHandle>,
	/// A list of game summaries.
	pub games: std::vec::Vec<GameSummary>,
	/// A list of party summaries.
	pub parties: std::vec::Vec<PartySummary>,
	/// A list of group summaries.
	pub suggested_groups: std::vec::Vec<GroupSummary>,
	/// A list of identity handles.
	pub suggested_players: std::vec::Vec<IdentityHandle>,
	/// Provided by watchable endpoints used in blocking loops.
	pub watch: WatchResponse,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CancelGameLinkResponse {}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CompleteGameLinkResponse {}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetGameLinkResponse {
	/// The link status between an identity and a game user.
	pub status: GameLinkStatus,
	/// A game handle.
	pub game: GameHandle,
	/// The current game user identity which created this game link.
	pub current_identity: IdentityHandle,
	/// If `status` is `GameLinkStatus$COMPLETE`, this will return the new identity and identity token to use for all future requests.
	pub new_identity: std::option::Option<GetGameLinkNewIdentity>,
	/// Provided by watchable endpoints used in blocking loops.
	pub watch: WatchResponse,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PrepareGameLinkResponse {
	/// Pass this to `rivet.api.identity#GetGameLink` to get the linking status. Valid for 15 minutes.
	pub identity_link_token: std::string::String,
	/// The URL that the user should visit to link their Rivet account.
	pub identity_link_url: std::string::String,
	/// Timestamp (in milliseconds) at which the link will expire.
	pub expire_ts: chrono::DateTime<chrono::Utc>,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct WatchEventsResponse {
	/// A list of global events. Ordered old to new.
	pub events: std::vec::Vec<GlobalEvent>,
	/// Provided by watchable endpoints used in blocking loops.
	pub watch: WatchResponse,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct UnmarkDeletionResponse {}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct MarkDeletionResponse {}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ListMutualFriendsResponse {
	/// A list of identity handles.
	pub identities: std::vec::Vec<IdentityHandle>,
	#[allow(missing_docs)] // documentation missing in model
	pub anchor: std::option::Option<std::string::String>,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RecentFollowerIgnoreResponse {}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ListRecentFollowersResponse {
	/// A list of identity handles.
	pub identities: std::vec::Vec<IdentityHandle>,
	#[allow(missing_docs)] // documentation missing in model
	pub anchor: std::option::Option<std::string::String>,
	/// Provided by watchable endpoints used in blocking loops.
	pub watch: WatchResponse,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetIdentitySummariesResponse {
	/// A list of identity summaries.
	pub identities: std::vec::Vec<IdentitySummary>,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetIdentityHandlesResponse {
	/// A list of identity handles.
	pub identities: std::vec::Vec<IdentityHandle>,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ReportIdentityResponse {}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ListFriendsResponse {
	/// A list of identity handles.
	pub identities: std::vec::Vec<IdentityHandle>,
	#[allow(missing_docs)] // documentation missing in model
	pub anchor: std::option::Option<std::string::String>,
	/// Provided by watchable endpoints used in blocking loops.
	pub watch: WatchResponse,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ListFollowingResponse {
	/// A list of identity handles.
	pub identities: std::vec::Vec<IdentityHandle>,
	#[allow(missing_docs)] // documentation missing in model
	pub anchor: std::option::Option<std::string::String>,
	/// Provided by watchable endpoints used in blocking loops.
	pub watch: WatchResponse,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ListFollowersResponse {
	/// A list of identity handles.
	pub identities: std::vec::Vec<IdentityHandle>,
	#[allow(missing_docs)] // documentation missing in model
	pub anchor: std::option::Option<std::string::String>,
	/// Provided by watchable endpoints used in blocking loops.
	pub watch: WatchResponse,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SignupForBetaResponse {}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CompleteIdentityAvatarUploadResponse {}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PrepareIdentityAvatarUploadResponse {
	/// A universally unique identifier.
	pub upload_id: std::string::String,
	/// A presigned request used to upload files. Upload your file to the given URL via a PUT request.
	pub presigned_request: UploadPresignedRequest,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct UnfollowIdentityResponse {}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct FollowIdentityResponse {}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct UpdateIdentityStatusResponse {}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RemoveIdentityGameActivityResponse {}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SetIdentityGameActivityResponse {}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SearchIdentitiesResponse {
	/// A list of identity handles.
	pub identities: std::vec::Vec<IdentityHandle>,
	#[allow(missing_docs)] // documentation missing in model
	pub anchor: std::option::Option<std::string::String>,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ValidateIdentityProfileResponse {
	/// A list of validation errors.
	pub errors: std::vec::Vec<ValidationError>,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct UpdateIdentityProfileResponse {}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetIdentitySelfProfileResponse {
	/// An identity profile.
	pub identity: IdentityProfile,
	/// Provided by watchable endpoints used in blocking loops.
	pub watch: WatchResponse,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetIdentityProfileResponse {
	/// An identity profile.
	pub identity: IdentityProfile,
	/// Provided by watchable endpoints used in blocking loops.
	pub watch: WatchResponse,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SetupIdentityResponse {
	/// Token used to authenticate the identity. Should be stored somewhere permanent. Pass this to `rivet.api.identity#SetupIdentity$existing_identity_token` next time `rivet.api.identity#SetupIdentity` is called. Token has a 90 day TTL. This means that if `rivet.api.identity#SetupIdentity` is not called again within 90 days, the token will no longer be valid. If this happens, the user can recover their account through the linking process (see `rivet.api.identity#PrepareGameLink`). This token should be stored locally and never sent to a server or another device. If this token is compromised, anyone with access to this token has control of the identity.
	pub identity_token: std::string::String,
	/// Timestamp (in milliseconds) at which the token expires.
	pub identity_token_expire_ts: chrono::DateTime<chrono::Utc>,
	/// Information about the identity that was just authenticated.
	pub identity: IdentityProfile,
	/// A universally unique identifier.
	pub game_id: std::string::String,
}

