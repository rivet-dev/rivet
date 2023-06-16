pub mod key;

#[derive(Debug, PartialEq, strum::FromRepr)]
#[repr(u8)]
pub enum PartyState {
	/// No party state.
	Idle = 0,

	/// Lobby find is pending.
	MatchmakerFindingLobby = 1,

	/// Party is in lobby.
	MatchmakerLobby = 2,
}

#[derive(Debug, PartialEq, strum::FromRepr)]
#[repr(u8)]
pub enum MemberState {
	/// Member has no state.
	Inactive = 0,

	/// Member is waiting to join the party's lobby.
	///
	/// This will happen if:
	/// * Joined a party in a `MatchmakerFindingLobby` state.
	/// * Join a party in a full lobby.
	/// * The lobby find failed for any other reason.
	///
	/// A direct lobby find will be attempted again when:
	/// * Player leaves the lobby, will attempt to fill the open space.
	/// * The party's lobby find completes, will attempt to join the same lobby.
	MatchmakerReady = 1,

	/// Member is finding lobby.
	///
	/// Note the difference between `MatchmakerFindingLobby` and
	/// `MatchmakerFindingLobbyDirect`.
	///
	/// This is not necessarily the same as
	/// `PartyState::MatchmakerFindingLobby`. See
	/// `MemberState::MatchmakerReady` for cases where the player will not be
	/// able to find a lobby.
	MatchmakerFindingLobby = 2,

	/// Member is finding a lobby independently of the party itself.
	///
	/// This happens when a player joins a party that's already in a lobby and
	/// has to directly join the lobby separately.
	MatchmakerFindingLobbyDirect = 3,

	/// Member is in a lobby.
	MatchmakerLobby = 4,
}
