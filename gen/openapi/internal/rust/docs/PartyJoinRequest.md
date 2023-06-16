# PartyJoinRequest

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**invite** | [**crate::models::PartyJoinInvite**](PartyJoinInvite.md) |  | 
**matchmaker_auto_join_lobby** | Option<**bool**> | Whether or not to automatically join the game lobby if a party is currently in game. | [optional]
**matchmaker_current_player_token** | Option<**String**> | If the player is currently in the lobby, pass the token from `rivet.matchmaker#MatchmakerLobbyJoinInfoPlayer$token`. This will prevent issuing a new player token. | [optional]

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


