#[allow(unused_imports)]
use chrono;
#[allow(unused_imports)]
use serde::{Deserialize, Serialize};

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

/// External links for this group.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GroupExternalLinks {
	/// A link to this group's profile page.
	pub profile: std::string::String,
	/// A link to this group's chat page.
	pub chat: std::string::String,
}

/// Provided by watchable endpoints used in blocking loops.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct WatchResponse {
	/// Index indicating the version of the data responded. Pas this to `rivet.common#WatchQuery` to block and wait for the next response.
	pub index: std::string::String,
}

/// A banned identity.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GroupBannedIdentity {
	/// An identity handle.
	pub identity: IdentityHandle,
	/// RFC3339 timestamp.
	pub ban_ts: chrono::DateTime<chrono::Utc>,
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

/// A presigned request used to upload files. Upload your file to the given URL via a PUT request.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct UploadPresignedRequest {
	/// The name of the file to upload. This is the same as the one given in the upload prepare file.
	pub path: std::string::String,
	/// The URL of the presigned request for which to upload your file to.
	pub url: std::string::String,
}

/// An error given by failed content validation.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ValidationError {
	/// A list of strings denoting the origin of a validation error.
	pub path: std::vec::Vec<std::string::String>,
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

/// A group join request.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GroupJoinRequest {
	/// An identity handle.
	pub identity: IdentityHandle,
	/// RFC3339 timestamp.
	pub ts: chrono::DateTime<chrono::Utc>,
}

/// A group member.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GroupMember {
	/// An identity handle.
	pub identity: IdentityHandle,
}

/// A list of group profiles.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GroupProfile {
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
	/// A list of group members.
	pub members: std::vec::Vec<GroupMember>,
	/// A list of group join requests.
	pub join_requests: std::vec::Vec<GroupJoinRequest>,
	/// Whether or not the current identity is currently requesting to join this group.
	pub is_current_identity_requesting_join: bool,
	/// A universally unique identifier.
	pub thread_id: std::option::Option<std::string::String>,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ResolveGroupJoinRequestRequest {
	#[allow(missing_docs)] // documentation missing in model
	pub resolution: bool,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CreateGroupJoinRequestRequest {}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetGroupInviteRequest {}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ConsumeGroupInviteRequest {}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CreateGroupInviteRequest {
	/// How long until the group invite expires (in milliseconds).
	pub ttl: std::option::Option<i64>,
	/// How many times the group invite can be used.
	pub use_count: std::option::Option<i64>,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetGroupBansRequest {}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct UnbanGroupIdentityRequest {}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct BanGroupIdentityRequest {}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct KickGroupMemberRequest {}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct LeaveGroupRequest {}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CompleteGroupAvatarUploadRequest {}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PrepareGroupAvatarUploadRequest {
	/// The path/filename of the group avatar.
	pub path: std::string::String,
	/// The MIME type of the group avatar.
	pub mime: std::option::Option<std::string::String>,
	/// Unsigned 64 bit integer.
	pub content_length: i64,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SearchGroupsRequest {}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TransferGroupOwnershipRequest {
	/// Idnetity to transfer the group to. Must be a member of the group.
	pub new_owner_identity_id: std::string::String,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ValidateGroupProfileRequest {
	/// Represent a resource's readable display name.
	pub display_name: std::option::Option<std::string::String>,
	/// Detailed information about a profile.
	pub bio: std::option::Option<std::string::String>,
	/// The current publicity value for the given group.
	pub publicity: std::option::Option<GroupPublicity>,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetGroupSummaryRequest {}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct UpdateGroupProfileRequest {
	/// Represent a resource's readable display name.
	pub display_name: std::option::Option<std::string::String>,
	/// Detailed information about a profile.
	pub bio: std::option::Option<std::string::String>,
	/// The current publicity value for the given group.
	pub publicity: std::option::Option<GroupPublicity>,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetGroupJoinRequestsRequest {}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetGroupMembersRequest {}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetGroupProfileRequest {}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CreateGroupRequest {
	/// Represent a resource's readable display name.
	pub display_name: std::string::String,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ListSuggestedGroupsRequest {}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ResolveGroupJoinRequestResponse {}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CreateGroupJoinRequestResponse {}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetGroupInviteResponse {
	/// A group handle.
	pub group: GroupHandle,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ConsumeGroupInviteResponse {
	/// A universally unique identifier.
	pub group_id: std::option::Option<std::string::String>,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CreateGroupInviteResponse {
	/// The code that will be passed to `rivet.api.group#ConsumeGroupInvite` to join a group.
	pub code: std::string::String,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetGroupBansResponse {
	/// A list of banned group members.
	pub banned_identities: std::vec::Vec<GroupBannedIdentity>,
	/// The pagination anchor.
	pub anchor: std::option::Option<std::string::String>,
	/// Provided by watchable endpoints used in blocking loops.
	pub watch: WatchResponse,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct UnbanGroupIdentityResponse {}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct BanGroupIdentityResponse {}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct KickGroupMemberResponse {}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct LeaveGroupResponse {}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CompleteGroupAvatarUploadResponse {}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PrepareGroupAvatarUploadResponse {
	/// A universally unique identifier.
	pub upload_id: std::string::String,
	/// A presigned request used to upload files. Upload your file to the given URL via a PUT request.
	pub presigned_request: UploadPresignedRequest,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SearchGroupsResponse {
	/// A list of group handles.
	pub groups: std::vec::Vec<GroupHandle>,
	#[allow(missing_docs)] // documentation missing in model
	pub anchor: std::option::Option<std::string::String>,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TransferGroupOwnershipResponse {}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ValidateGroupProfileResponse {
	/// A list of validation errors.
	pub errors: std::vec::Vec<ValidationError>,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetGroupSummaryResponse {
	/// A group summary.
	pub group: GroupSummary,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct UpdateGroupProfileResponse {}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetGroupJoinRequestsResponse {
	/// A list of group join requests.
	pub join_requests: std::vec::Vec<GroupJoinRequest>,
	/// The pagination anchor.
	pub anchor: std::option::Option<std::string::String>,
	/// Provided by watchable endpoints used in blocking loops.
	pub watch: WatchResponse,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetGroupMembersResponse {
	/// A list of group members.
	pub members: std::vec::Vec<GroupMember>,
	/// The pagination anchor.
	pub anchor: std::option::Option<std::string::String>,
	/// Provided by watchable endpoints used in blocking loops.
	pub watch: WatchResponse,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetGroupProfileResponse {
	/// A list of group profiles.
	pub group: GroupProfile,
	/// Provided by watchable endpoints used in blocking loops.
	pub watch: WatchResponse,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CreateGroupResponse {
	/// A universally unique identifier.
	pub group_id: std::string::String,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ListSuggestedGroupsResponse {
	/// A list of group summaries.
	pub groups: std::vec::Vec<GroupSummary>,
	/// Provided by watchable endpoints used in blocking loops.
	pub watch: WatchResponse,
}

