# \MatchmakerLobbiesApi

All URIs are relative to *https://api.rivet.gg*

Method | HTTP request | Description
------------- | ------------- | -------------
[**matchmaker_lobbies_create**](MatchmakerLobbiesApi.md#matchmaker_lobbies_create) | **POST** /matchmaker/lobbies/create | 
[**matchmaker_lobbies_find**](MatchmakerLobbiesApi.md#matchmaker_lobbies_find) | **POST** /matchmaker/lobbies/find | 
[**matchmaker_lobbies_get_state**](MatchmakerLobbiesApi.md#matchmaker_lobbies_get_state) | **GET** /matchmaker/lobbies/{lobby_id}/state | 
[**matchmaker_lobbies_join**](MatchmakerLobbiesApi.md#matchmaker_lobbies_join) | **POST** /matchmaker/lobbies/join | 
[**matchmaker_lobbies_list**](MatchmakerLobbiesApi.md#matchmaker_lobbies_list) | **GET** /matchmaker/lobbies/list | 
[**matchmaker_lobbies_ready**](MatchmakerLobbiesApi.md#matchmaker_lobbies_ready) | **POST** /matchmaker/lobbies/ready | 
[**matchmaker_lobbies_set_closed**](MatchmakerLobbiesApi.md#matchmaker_lobbies_set_closed) | **PUT** /matchmaker/lobbies/closed | 
[**matchmaker_lobbies_set_state**](MatchmakerLobbiesApi.md#matchmaker_lobbies_set_state) | **PUT** /matchmaker/lobbies/state | 



## matchmaker_lobbies_create

> crate::models::MatchmakerCreateLobbyResponse matchmaker_lobbies_create(matchmaker_lobbies_create_request)


Creates a custom lobby.  When [tokenless authentication](/docs/general/concepts/tokenless-authentication/web) is enabled in your game namespace, this endpoint does not require a token to authenticate. Otherwise, a [development namespace token](/docs/general/concepts/token-types#namespace-development) can be used for mock responses and a [public namespace token](/docs/general/concepts/token-types#namespace-public) can be used for general authentication.

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


Finds a lobby based on the given criteria. If a lobby is not found and `prevent_auto_create_lobby` is `false`, a new lobby will be created.  When [tokenless authentication](/docs/general/concepts/tokenless-authentication/web) is enabled in your game namespace, this endpoint does not require a token to authenticate. Otherwise, a [development namespace token](/docs/general/concepts/token-types#namespace-development) can be used for mock responses and a [public namespace token](/docs/general/concepts/token-types#namespace-public) can be used for general authentication.

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


## matchmaker_lobbies_get_state

> serde_json::Value matchmaker_lobbies_get_state(lobby_id)


Get the state of any lobby.  This endpoint requires a [lobby token](/docs/general/concepts/token-types#matchmaker-lobby) for authentication, or a [development namespace token](/docs/general/concepts/token-types#namespace-development) for mock responses. When running on Rivet servers, you can access the given lobby token from the [`RIVET_TOKEN`](/docs/matchmaker/concepts/lobby-env) environment variable.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**lobby_id** | **uuid::Uuid** |  | [required] |

### Return type

[**serde_json::Value**](serde_json::Value.md)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## matchmaker_lobbies_join

> crate::models::MatchmakerJoinLobbyResponse matchmaker_lobbies_join(matchmaker_lobbies_join_request)


Joins a specific lobby. This request will use the direct player count configured for the lobby group.  When [tokenless authentication](/docs/general/concepts/tokenless-authentication/web) is enabled in your game namespace, this endpoint does not require a token to authenticate. Otherwise, a [development namespace token](/docs/general/concepts/token-types#namespace-development) can be used for mock responses and a [public namespace token](/docs/general/concepts/token-types#namespace-public) can be used for general authentication.

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

> crate::models::MatchmakerListLobbiesResponse matchmaker_lobbies_list(include_state)


Lists all open lobbies.  When [tokenless authentication](/docs/general/concepts/tokenless-authentication/web) is enabled in your game namespace, this endpoint does not require a token to authenticate. Otherwise, a [development namespace token](/docs/general/concepts/token-types#namespace-development) can be used for mock responses and a [public namespace token](/docs/general/concepts/token-types#namespace-public) can be used for general authentication.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**include_state** | Option<**bool**> |  |  |

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


Marks the current lobby as ready to accept connections. Players will not be able to connect to this lobby until the lobby is flagged as ready. This endpoint requires a [lobby token](/docs/general/concepts/token-types#matchmaker-lobby) for authentication, or a [development namespace token](/docs/general/concepts/token-types#namespace-development) for mock responses. When running on Rivet servers, you can access the given lobby token from the [`RIVET_TOKEN`](/docs/matchmaker/concepts/lobby-env) environment variable.

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


If `is_closed` is `true`, the matchmaker will no longer route players to the lobby. Players can still join using the /join endpoint (this can be disabled by the developer by rejecting all new connections after setting the lobby to closed). Does not shutdown the lobby.  This endpoint requires a [lobby token](/docs/general/concepts/token-types#matchmaker-lobby) for authentication, or a [development namespace token](/docs/general/concepts/token-types#namespace-development) for mock responses. When running on Rivet servers, you can access the given lobby token from the [`RIVET_TOKEN`](/docs/matchmaker/concepts/lobby-env) environment variable.

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


## matchmaker_lobbies_set_state

> matchmaker_lobbies_set_state(body)


Sets the state JSON of the current lobby.  This endpoint requires a [lobby token](/docs/general/concepts/token-types#matchmaker-lobby) for authentication, or a [development namespace token](/docs/general/concepts/token-types#namespace-development) for mock responses. When running on Rivet servers, you can access the given lobby token from the [`RIVET_TOKEN`](/docs/matchmaker/concepts/lobby-env) environment variable.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**body** | Option<**serde_json::Value**> |  |  |

### Return type

 (empty response body)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

