# \PartyActivityMatchmakerApi

All URIs are relative to *http://localhost*

Method | HTTP request | Description
------------- | ------------- | -------------
[**party_activity_matchmaker_find_lobby_for_party**](PartyActivityMatchmakerApi.md#party_activity_matchmaker_find_lobby_for_party) | **POST** /parties/self/activity/matchmaker/lobbies/find | 
[**party_activity_matchmaker_join_lobby_for_party**](PartyActivityMatchmakerApi.md#party_activity_matchmaker_join_lobby_for_party) | **POST** /parties/self/activity/matchmaker/lobbies/join | 
[**party_activity_matchmaker_request_player**](PartyActivityMatchmakerApi.md#party_activity_matchmaker_request_player) | **POST** /parties/self/members/self/matchmaker/request-player | 



## party_activity_matchmaker_find_lobby_for_party

> party_activity_matchmaker_find_lobby_for_party(party_activity_find_matchmaker_lobby_for_party_request)


Attempts to make the current identity's party find a lobby based on the given criteria. If succeeds, all party members will receive a `GlobalEventMatchmakerLobbyJoin` event with all the information required to join the lobby. This request will use the party player count configured for the lobby group. See `FindLobby`.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**party_activity_find_matchmaker_lobby_for_party_request** | [**PartyActivityFindMatchmakerLobbyForPartyRequest**](PartyActivityFindMatchmakerLobbyForPartyRequest.md) |  | [required] |

### Return type

 (empty response body)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## party_activity_matchmaker_join_lobby_for_party

> party_activity_matchmaker_join_lobby_for_party(party_activity_join_matchmaker_lobby_for_party_request)


Attempts to make the current identity's party join a specific matchmaker lobby. This request will use the party player count configured for the lobby group. If succeeds, all party members will receive a `GlobalEventMatchmakerLobbyJoin` event with all the information required to join the lobby. Identity must be the party leader. See `JoinLobby`.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**party_activity_join_matchmaker_lobby_for_party_request** | [**PartyActivityJoinMatchmakerLobbyForPartyRequest**](PartyActivityJoinMatchmakerLobbyForPartyRequest.md) |  | [required] |

### Return type

 (empty response body)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## party_activity_matchmaker_request_player

> party_activity_matchmaker_request_player()


### Parameters

This endpoint does not need any parameter.

### Return type

 (empty response body)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

