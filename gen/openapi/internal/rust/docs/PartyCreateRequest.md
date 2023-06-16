# PartyCreateRequest

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**invites** | Option<[**Vec<crate::models::PartyCreateInviteConfig>**](PartyCreateInviteConfig.md)> |  | [optional]
**matchmaker_current_player_token** | Option<**String**> | If the player is currently in the lobby, pass the token from `rivet.matchmaker#MatchmakerLobbyJoinInfoPlayer$token`. This will prevent issuing a new player token and automatically set the party state to the player's current lobby. | [optional]
**party_size** | Option<**f64**> | How many members can join the party. If using this party with the matchmaker, this number should be less than or equal to your party player limit. Super large parties may not be able to fit insite a lobby and be unable to join the game. | [optional]
**publicity** | Option<[**crate::models::PartyCreatePublicityConfig**](PartyCreatePublicityConfig.md)> |  | [optional]

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


