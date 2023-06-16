#[allow(unused_imports)]
use chrono;
#[allow(unused_imports)]
use serde::{Deserialize, Serialize};

/// Represents a value for which notification service to unregister.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Hash, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum NotificationUnregisterService {
	/// Firebase service.
	Firebase,
	/// Unknown contains new variants that have been added since this code was generated.
	Unknown(String),
}
impl std::convert::From<&str> for NotificationUnregisterService {
	fn from(s: &str) -> Self {
		match s {
			"firebase" => NotificationUnregisterService::Firebase,
			other => NotificationUnregisterService::Unknown(other.to_owned()),
		}
	}
}
impl std::str::FromStr for NotificationUnregisterService {
	type Err = std::convert::Infallible;

	fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
		Ok(NotificationUnregisterService::from(s))
	}
}
impl NotificationUnregisterService {
	/// Returns the `&str` value of the enum member.
	pub fn as_str(&self) -> &str {
		match self {
			NotificationUnregisterService::Firebase => "firebase",
			NotificationUnregisterService::Unknown(s) => s.as_ref(),
		}
	}
	/// Returns all the `&str` values of the enum members.
	pub fn values() -> &'static [&'static str] {
		&["firebase"]
	}
}
impl AsRef<str> for NotificationUnregisterService {
	fn as_ref(&self) -> &str {
		self.as_str()
	}
}

/// A union representing which notification service to register.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum NotificationRegisterService {
	/// Represents push notification configuration for Firebase.
	Firebase(NotificationRegisterFirebaseService),
}

/// Represents push notification configuration for Firebase.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct NotificationRegisterFirebaseService {
	#[allow(missing_docs)] // documentation missing in model
	pub access_key: std::string::String,
}

/// Provided by watchable endpoints used in blocking loops.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct WatchResponse {
	/// Index indicating the version of the data responded. Pas this to `rivet.common#WatchQuery` to block and wait for the next response.
	pub index: std::string::String,
}

/// A game profile.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GameProfile {
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
	/// A group summary.
	pub developer: GroupSummary,
	/// A list of game tags.
	pub tags: std::vec::Vec<std::string::String>,
	/// A description of the given game.
	pub description: std::string::String,
	/// A list of platform links.
	pub platforms: std::vec::Vec<GamePlatformLink>,
	/// A list of group summaries.
	pub recommended_groups: std::vec::Vec<GroupSummary>,
	/// A list of game leaderboard categories.
	pub identity_leaderboard_categories: std::vec::Vec<GameLeaderboardCategory>,
	/// A list of game leaderboard categories.
	pub group_leaderboard_categories: std::vec::Vec<GameLeaderboardCategory>,
}

/// A game leaderboard category.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GameLeaderboardCategory {
	/// Represent a resource's readable display name.
	pub display_name: std::string::String,
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

/// A platform link denoting a supported platform.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GamePlatformLink {
	/// Represent a resource's readable display name.
	pub display_name: std::string::String,
	/// The URL to the given game's method of distribution on this platform.
	pub url: std::string::String,
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
pub struct UnregisterNotificationsRequest {}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RegisterNotificationsRequest {
	/// A union representing which notification service to register.
	pub service: NotificationRegisterService,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ResolveBetaJoinRequestRequest {
	/// Whether or not to accept the beta join request.
	pub resolution: bool,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetGameProfileRequest {}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetSuggestedGamesRequest {}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct UnregisterNotificationsResponse {}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RegisterNotificationsResponse {}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ResolveBetaJoinRequestResponse {}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetGameProfileResponse {
	/// A game profile.
	pub game: GameProfile,
	/// Provided by watchable endpoints used in blocking loops.
	pub watch: WatchResponse,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetSuggestedGamesResponse {
	/// A list of game summaries.
	pub games: std::vec::Vec<GameSummary>,
	/// Provided by watchable endpoints used in blocking loops.
	pub watch: WatchResponse,
}

