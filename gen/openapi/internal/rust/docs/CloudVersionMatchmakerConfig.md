# CloudVersionMatchmakerConfig

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**captcha** | Option<[**crate::models::CloudVersionMatchmakerCaptcha**](CloudVersionMatchmakerCaptcha.md)> |  | [optional]
**dev_hostname** | Option<**String**> | Client-side configuration | [optional]
**docker** | Option<[**crate::models::CloudVersionMatchmakerGameModeRuntimeDocker**](CloudVersionMatchmakerGameModeRuntimeDocker.md)> |  | [optional]
**game_modes** | Option<[**::std::collections::HashMap<String, crate::models::CloudVersionMatchmakerGameMode>**](CloudVersionMatchmakerGameMode.md)> | A list of game modes. | [optional]
**idle_lobbies** | Option<[**crate::models::CloudVersionMatchmakerGameModeIdleLobbiesConfig**](CloudVersionMatchmakerGameModeIdleLobbiesConfig.md)> |  | [optional]
**lobby_groups** | Option<[**Vec<crate::models::CloudVersionMatchmakerLobbyGroup>**](CloudVersionMatchmakerLobbyGroup.md)> | **Deprecated: use `game_modes` instead** A list of game modes. | [optional]
**max_players** | Option<**i32**> |  | [optional]
**max_players_direct** | Option<**i32**> |  | [optional]
**max_players_party** | Option<**i32**> |  | [optional]
**regions** | Option<[**::std::collections::HashMap<String, crate::models::CloudVersionMatchmakerGameModeRegion>**](CloudVersionMatchmakerGameModeRegion.md)> |  | [optional]
**tier** | Option<**String**> |  | [optional]

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


