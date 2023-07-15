# \MatchmakerLobbiesApi

All URIs are relative to *http://localhost*

Method | HTTP request | Description
------------- | ------------- | -------------
[**matchmaker_lobbies_create**](MatchmakerLobbiesApi.md#matchmaker_lobbies_create) | **POST** /lobbies/create | 
[**matchmaker_lobbies_find**](MatchmakerLobbiesApi.md#matchmaker_lobbies_find) | **POST** /lobbies/find | 
[**matchmaker_lobbies_join**](MatchmakerLobbiesApi.md#matchmaker_lobbies_join) | **POST** /lobbies/join | 
[**matchmaker_lobbies_list**](MatchmakerLobbiesApi.md#matchmaker_lobbies_list) | **GET** /lobbies/list | 
[**matchmaker_lobbies_ready**](MatchmakerLobbiesApi.md#matchmaker_lobbies_ready) | **POST** /lobbies/ready | 
[**matchmaker_lobbies_set_closed**](MatchmakerLobbiesApi.md#matchmaker_lobbies_set_closed) | **PUT** /lobbies/closed | 



## matchmaker_lobbies_create

> crate::models::MatchmakerCreateLobbyResponse matchmaker_lobbies_create(matchmaker_lobbies_create_request)


Creates a custom lobby. 

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**matchmaker_lobbies_create_request** | [**MatchmakerLobbiesCreateRequest**](MatchmakerLobbiesCreateRequest.md) |  | [required] |

### Return type

[**crate::models::MatchmakerCreateLobbyResponse**](MatchmakerCreateLobbyResponse.md)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## matchmaker_lobbies_find

> crate::models::MatchmakerFindLobbyResponse matchmaker_lobbies_find(matchmaker_lobbies_find_request, origin)


Finds a lobby based on the given criteria. If a lobby is not found and `prevent_auto_create_lobby` is `true`,  a new lobby will be created. 

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**matchmaker_lobbies_find_request** | [**MatchmakerLobbiesFindRequest**](MatchmakerLobbiesFindRequest.md) |  | [required] |
**origin** | Option<**String**> |  |  |

### Return type

[**crate::models::MatchmakerFindLobbyResponse**](MatchmakerFindLobbyResponse.md)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## matchmaker_lobbies_join

> crate::models::MatchmakerJoinLobbyResponse matchmaker_lobbies_join(matchmaker_lobbies_join_request)


Joins a specific lobby. This request will use the direct player count configured for the lobby group. 

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**matchmaker_lobbies_join_request** | [**MatchmakerLobbiesJoinRequest**](MatchmakerLobbiesJoinRequest.md) |  | [required] |

### Return type

[**crate::models::MatchmakerJoinLobbyResponse**](MatchmakerJoinLobbyResponse.md)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## matchmaker_lobbies_list

> crate::models::MatchmakerListLobbiesResponse matchmaker_lobbies_list()


Lists all open lobbies.

### Parameters

This endpoint does not need any parameter.

### Return type

[**crate::models::MatchmakerListLobbiesResponse**](MatchmakerListLobbiesResponse.md)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## matchmaker_lobbies_ready

> matchmaker_lobbies_ready()


Marks the current lobby as ready to accept connections.  Players will not be able to connect to this lobby until the  lobby is flagged as ready.

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


## matchmaker_lobbies_set_closed

> matchmaker_lobbies_set_closed(matchmaker_lobbies_set_closed_request)


If `is_closed` is `true`, players will be prevented from joining the lobby. Does not shutdown the lobby. 

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**matchmaker_lobbies_set_closed_request** | [**MatchmakerLobbiesSetClosedRequest**](MatchmakerLobbiesSetClosedRequest.md) |  | [required] |

### Return type

 (empty response body)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

