# CloudVersionMatchmakerLobbyGroup

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**max_players_direct** | **i32** | Unsigned 32 bit integer. | 
**max_players_normal** | **i32** | Unsigned 32 bit integer. | 
**max_players_party** | **i32** | Unsigned 32 bit integer. | 
**name_id** | **String** | **Deprecated: use GameMode instead** A human readable short identifier used to references resources. Different than a `rivet.common#Uuid` because this is intended to be human readable. Different than `rivet.common#DisplayName` because this should not include special characters and be short. | 
**regions** | [**Vec<crate::models::CloudVersionMatchmakerLobbyGroupRegion>**](CloudVersionMatchmakerLobbyGroupRegion.md) | A list of game mode regions. | 
**runtime** | [**crate::models::CloudVersionMatchmakerLobbyGroupRuntime**](CloudVersionMatchmakerLobbyGroupRuntime.md) |  | 

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


