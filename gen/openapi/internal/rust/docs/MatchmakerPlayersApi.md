# \MatchmakerPlayersApi

All URIs are relative to *http://localhost*

Method | HTTP request | Description
------------- | ------------- | -------------
[**matchmaker_players_connected**](MatchmakerPlayersApi.md#matchmaker_players_connected) | **POST** /players/connected | 
[**matchmaker_players_disconnected**](MatchmakerPlayersApi.md#matchmaker_players_disconnected) | **POST** /players/disconnected | 
[**matchmaker_players_get_statistics**](MatchmakerPlayersApi.md#matchmaker_players_get_statistics) | **GET** /players/statistics | 



## matchmaker_players_connected

> matchmaker_players_connected(matchmaker_players_connected_request)


Validates the player token is valid and has not already been consumed then marks the player as connected. # Player Tokens and Reserved Slots Player tokens reserve a spot in the lobby until they expire. This allows for precise matchmaking up to exactly the lobby's player limit, which is important for games with small lobbies and a high influx of players. By calling this endpoint with the player token, the player's spot is marked as connected and will not expire. If this endpoint is never called, the player's token will expire and this spot will be filled by another player. # Anti-Botting Player tokens are only issued by caling `lobbies.join`, calling `lobbies.find`, or from the `GlobalEventMatchmakerLobbyJoin` event. These endpoints have anti-botting measures (i.e. enforcing max player limits, captchas, and detecting bots), so valid player tokens provide some confidence that the player is not a bot. Therefore, it's important to make sure the token is valid by waiting for this endpoint to return OK before allowing the connected socket to do anything else. If this endpoint returns an error, the socket should be disconnected immediately. # How to Transmit the Player Token The client is responsible for acquiring the player token by caling `lobbies.join`, calling `lobbies.find`, or from the `GlobalEventMatchmakerLobbyJoin` event.  Beyond that, it's up to the developer how the player token is transmitted to the lobby. If using WebSockets, the player token can be transmitted as a query parameter. Otherwise, the player token will likely be automatically sent by the client once the socket opens. As mentioned above, nothing else should happen until the player token is validated. 

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**matchmaker_players_connected_request** | [**MatchmakerPlayersConnectedRequest**](MatchmakerPlayersConnectedRequest.md) |  | [required] |

### Return type

 (empty response body)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## matchmaker_players_disconnected

> matchmaker_players_disconnected(matchmaker_players_connected_request)


Marks a player as disconnected. # Ghost Players If players are not marked as disconnected, lobbies will result with \"ghost players\" that the matchmaker thinks exist but are no longer connected to the lobby.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**matchmaker_players_connected_request** | [**MatchmakerPlayersConnectedRequest**](MatchmakerPlayersConnectedRequest.md) |  | [required] |

### Return type

 (empty response body)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## matchmaker_players_get_statistics

> crate::models::MatchmakerGetStatisticsResponse matchmaker_players_get_statistics()


Gives matchmaker statistics about the players in game.

### Parameters

This endpoint does not need any parameter.

### Return type

[**crate::models::MatchmakerGetStatisticsResponse**](MatchmakerGetStatisticsResponse.md)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

