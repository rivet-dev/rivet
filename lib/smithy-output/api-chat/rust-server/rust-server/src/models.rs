#[allow(unused_imports)]
use chrono;
#[allow(unused_imports)]
use serde::{Deserialize, Serialize};

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

/// Data to send in a chat message.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SendMessageBody {
	/// `SendMessageBody` variant for party invite messages. Cannot send to party topics.
	PartyInvite(SendMessageBodyPartyInvite),
	/// `rivet.api.chat.common#SendMessageBody` variant for text messages.
	Text(SendMessageBodyText),
}

/// `SendMessageBody` variant for party invite messages. Cannot send to party topics.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SendMessageBodyPartyInvite {
	/// An invite token.
	pub token: std::string::String,
}

/// `rivet.api.chat.common#SendMessageBody` variant for text messages.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SendMessageBodyText {
	#[allow(missing_docs)] // documentation missing in model
	pub body: std::string::String,
}

/// Topic to send a chat message to. If you already know the thread ID, use `thread_id`.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SendChatTopic {
	/// A universally unique identifier.
	GroupId(std::string::String),
	/// A universally unique identifier.
	IdentityId(std::string::String),
	/// A universally unique identifier.
	PartyId(std::string::String),
	/// A universally unique identifier.
	ThreadId(std::string::String),
}

/// Represents a chat typing status.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ChatTypingStatus {
	/// Not typing.
	Idle(Unit),
	/// Typing.
	Typing(Unit),
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Unit {}

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

/// Represents which direction to query messages from relative to the given
/// timestamp.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Hash, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum QueryDirection {
	/// Query messages after given timestamp.
	After,
	/// Query messages send before given timestamp.
	Before,
	/// Query messages before and after the given timestamp. This will return at most `count * 2` messages.
	BeforeAndAfter,
	/// Unknown contains new variants that have been added since this code was generated.
	Unknown(String),
}
impl std::convert::From<&str> for QueryDirection {
	fn from(s: &str) -> Self {
		match s {
			"after" => QueryDirection::After,
			"before" => QueryDirection::Before,
			"before_and_after" => QueryDirection::BeforeAndAfter,
			other => QueryDirection::Unknown(other.to_owned()),
		}
	}
}
impl std::str::FromStr for QueryDirection {
	type Err = std::convert::Infallible;

	fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
		Ok(QueryDirection::from(s))
	}
}
impl QueryDirection {
	/// Returns the `&str` value of the enum member.
	pub fn as_str(&self) -> &str {
		match self {
			QueryDirection::After => "after",
			QueryDirection::Before => "before",
			QueryDirection::BeforeAndAfter => "before_and_after",
			QueryDirection::Unknown(s) => s.as_ref(),
		}
	}
	/// Returns all the `&str` values of the enum members.
	pub fn values() -> &'static [&'static str] {
		&["after", "before", "before_and_after"]
	}
}
impl AsRef<str> for QueryDirection {
	fn as_ref(&self) -> &str {
		self.as_str()
	}
}

/// Provided by watchable endpoints used in blocking loops.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct WatchResponse {
	/// Index indicating the version of the data responded. Pas this to `rivet.common#WatchQuery` to block and wait for the next response.
	pub index: std::string::String,
}

/// The chat typing status of an identity.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ChatIdentityTypingStatus {
	/// An identity handle.
	pub identity: IdentityHandle,
	/// Represents a chat typing status.
	pub status: ChatTypingStatus,
}

/// Represents a topic of the given chat thread without the associated handles for the topic.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ChatSimpleTopic {
	/// `rivet.chat#ChatSimpleTopic` variant for direct (identity to identity) chats.
	Direct(ChatSimpleTopicDirect),
	/// `rivet.chat#ChatSimpleTopic` variant for groups.
	Group(ChatSimpleTopicGroup),
	/// `rivet.chat#ChatSimpleTopic` variant for parties.
	Party(ChatSimpleTopicParty),
}

/// `rivet.chat#ChatSimpleTopic` variant for direct (identity to identity) chats.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ChatSimpleTopicDirect {
	/// A universally unique identifier.
	pub identity_a_id: std::string::String,
	/// A universally unique identifier.
	pub identity_b_id: std::string::String,
}

/// `rivet.chat#ChatSimpleTopic` variant for parties.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ChatSimpleTopicParty {
	/// A universally unique identifier.
	pub party_id: std::string::String,
}

/// `rivet.chat#ChatSimpleTopic` variant for groups.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ChatSimpleTopicGroup {
	/// A universally unique identifier.
	pub group_id: std::string::String,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetDirectThreadRequest {}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SendChatMessageRequest {
	/// Topic to send a chat message to. If you already know the thread ID, use `thread_id`.
	pub topic: SendChatTopic,
	/// Data to send in a chat message.
	pub message_body: SendMessageBody,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SetTypingStatusRequest {
	/// Represents a chat typing status.
	pub status: ChatTypingStatus,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SetThreadReadRequest {
	/// Any messages newer than this timestamp will be marked as unread. This should be the current timestamp (in milliseconds).
	pub last_read_ts: chrono::DateTime<chrono::Utc>,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetThreadHistoryRequest {}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct WatchThreadRequest {}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetThreadTopicRequest {}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetDirectThreadResponse {
	/// A universally unique identifier.
	pub thread_id: std::option::Option<std::string::String>,
	/// An identity handle.
	pub identity: std::option::Option<IdentityHandle>,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SendChatMessageResponse {
	/// A universally unique identifier.
	pub chat_message_id: std::string::String,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SetTypingStatusResponse {}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SetThreadReadResponse {}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetThreadHistoryResponse {
	/// Ordered old to new. If querying `rivet.api.chat.common#before_and_after`, this will be `count * 2` long.
	pub chat_messages: std::vec::Vec<ChatMessage>,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct WatchThreadResponse {
	/// All messages new messages posted to this thread. Ordered old to new.
	pub chat_messages: std::vec::Vec<ChatMessage>,
	/// All identities that are currently typing in this thread.
	pub typing_statuses: std::option::Option<std::vec::Vec<ChatIdentityTypingStatus>>,
	/// Provided by watchable endpoints used in blocking loops.
	pub watch: WatchResponse,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetThreadTopicResponse {
	/// Represents a topic of the given chat thread without the associated handles for the topic.
	pub topic: ChatSimpleTopic,
}

