# Docs

## Types of party states

See `Party.state` in `proto/backend/party.proto` and `party_config::State` in `lib/util/party/src/key.rs`.

-   `Idle`: The party is not doing anything. For example, the game is sitting in the game menu or players are hanging out on the hub.
-   `MatchmakerFindingLobby`: There is a find request in progress for the lobby. If the find request fails, it will go back to `Idle`. If the find request succeeds, it will go to `MatchmakerLobby`.
-   `MatchmakerLobby`: The party is in a lobby. This does not mean that all of the party members are in the lobby, see the member-specific states.

## Types of party member states

See `PartyMember.state` in `proto/backend/party.proto` and `party_member_config::State` in `lib/util/party/src/key.rs`.

-   `Inactive`: The player is not doing anything. For example, the player can be sitting in the game menu or hanging out on the hub.
    -   It's possible for the member to be in an inactive state if the party is in a lobby; this means the player does not want to be playing the game. For example, some players could be playing a game while another player is hanging out on the hub, maybe on their phone and just wants to interact with the other players.
-   `MatchmakerReady`: This means the member wants a player created for them.
    -   Members can be in the ready state while the party is in an idle state. This means that the player will get a player created for them
    -   Members can be in the ready state while the party is in a lobby. This means that the player could not join the lobby because it is full or the player left the lobby. Calling `msg-party-member-resolve` will attempt to create a player for the member again.
-   `MatchmakerFindingLobby`: A find request is in progress for the member. This is _specifically_ for find requests created for the party as a whole. i.e. called from `party-state-mm-lobby-find`
-   `MatchmakerFindingLobbyDirect`: A find request is in progress independently for this member. This state is reached when a party member requests a player for a party that is already in a lobby, so a join request is created directly to the given lobby.
-   `MatchmakerLobby`: The member is in a lobby.

## Ways users can join a lobby

-   `party-state-mm-lobby-find` will create a find request for all players.
-   `party-member-state-resolve` will create a find request for the player if in the `MatchmakerReady` state and the lobby is already in a lobby.

## Resolving party member states

The party state can be thought of as the desired state of all party members. The party member state is what each party member is actually doing. Calling `msg-party-member-state-resolve` will update the party member's state to be in the desired state.

For example, if a party member is in `MatchmakerReady` state, resolving the state will attempt to join a lobby and put the player in the `MatchmakerFindingLobbyDirect` state.

## Flow from user joining party -> joining lobby

### Joining a lobby as party leader

1. User is already in party and is leader
1. `POST party.api.rivet.gg/v1/parties/self/members/self/matchmaker/find` or `POST party.api.rivet.gg/v1/parties/self/members/self/matchmaker/join`
1. `msg-party-state-mm-lobby-find` -> `party-state-mm-lobby-find` -> `msg-mm-lobby-find` -> _...matchmaker flow..._ -> `msg-mm-lobby-find-complete` -> `party-state-mm-lobby-find-complete` -> `msg-user-mm-lobby-join`
1. `GET identity.api.rivet.gg/v1/events/live` watch request returns new lobby join info

### Joining a party already in a lobby

1. `POST party.api.rivet.gg/v1/parties/join`
1. `msg-party-member-create` -> `party-member-create` -> `msg-party-member-create-complete`
1. `POST party.api.rivet.gg/v1//parties/self/members/self/matchmaker/ready`
1. `msg-party-member-state-set-mm-pending` -> `party-member-state-set-mm-pending` -> `msg-party-member-state-resolve` -> `party-member-state-resolve` -> `msg-mm-lobby-find` -> _...matchmaker flow..._ -> `msg-mm-lobby-find-complete` -> `party-member-state-mm-lobby-find-complete` -> `msg-user-mm-lobby-join`
1. `GET identity.api.rivet.gg/v1/events/live` watch request returns new lobby join info

## Party invites

Party invites can be created either in `POST party.api.rivet.gg/v1/parties` or `POST party.api.rivet.gg/v1/parties/self/invites`.

## Aliases

Invites can optionally contain an alias. An alias is a user-provided string that users can use to join a party. Party aliases are unique within the game's namespace and can only be consumed within the same namespace.

## Ways to join a party

See documentation for `rivet.api.party.common#JoinPartyInvite`.
