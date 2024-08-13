# \GamesServersBuildsApi

All URIs are relative to *https://api.rivet.gg*

Method | HTTP request | Description
------------- | ------------- | -------------
[**games_servers_builds_complete_build**](GamesServersBuildsApi.md#games_servers_builds_complete_build) | **POST** /games/{game_id}/builds/{build_id}/complete | 
[**games_servers_builds_list_builds**](GamesServersBuildsApi.md#games_servers_builds_list_builds) | **GET** /games/{game_id}/builds | 
[**games_servers_builds_prepare_build**](GamesServersBuildsApi.md#games_servers_builds_prepare_build) | **POST** /games/{game_id}/builds/prepare | 



## games_servers_builds_complete_build

> games_servers_builds_complete_build(game_id, build_id)


Marks an upload as complete.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**game_id** | **uuid::Uuid** |  | [required] |
**build_id** | **uuid::Uuid** |  | [required] |

### Return type

 (empty response body)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## games_servers_builds_list_builds

> crate::models::GamesServersListBuildsResponse games_servers_builds_list_builds(game_id, tags, game_id2)


Lists all builds of the game associated with the token used. Can be filtered by tags in the query string.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**game_id** | **uuid::Uuid** |  | [required] |
**tags** | Option<**String**> |  |  |
**game_id2** | Option<**uuid::Uuid**> |  |  |

### Return type

[**crate::models::GamesServersListBuildsResponse**](GamesServersListBuildsResponse.md)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## games_servers_builds_prepare_build

> crate::models::GamesServersCreateBuildResponse games_servers_builds_prepare_build(game_id, games_servers_create_build_request)


Creates a new game build for the given game.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**game_id** | **uuid::Uuid** |  | [required] |
**games_servers_create_build_request** | [**GamesServersCreateBuildRequest**](GamesServersCreateBuildRequest.md) |  | [required] |

### Return type

[**crate::models::GamesServersCreateBuildResponse**](GamesServersCreateBuildResponse.md)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

