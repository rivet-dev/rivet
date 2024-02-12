# \CloudGamesBuildsApi

All URIs are relative to *https://api.rivet.gg*

Method | HTTP request | Description
------------- | ------------- | -------------
[**cloud_games_builds_create_game_build**](CloudGamesBuildsApi.md#cloud_games_builds_create_game_build) | **POST** /cloud/games/{game_id}/builds | 
[**cloud_games_builds_list_game_builds**](CloudGamesBuildsApi.md#cloud_games_builds_list_game_builds) | **GET** /cloud/games/{game_id}/builds | 



## cloud_games_builds_create_game_build

> crate::models::CloudGamesCreateGameBuildResponse cloud_games_builds_create_game_build(game_id, cloud_games_create_game_build_request)


Creates a new game build for the given game.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**game_id** | **uuid::Uuid** |  | [required] |
**cloud_games_create_game_build_request** | [**CloudGamesCreateGameBuildRequest**](CloudGamesCreateGameBuildRequest.md) |  | [required] |

### Return type

[**crate::models::CloudGamesCreateGameBuildResponse**](CloudGamesCreateGameBuildResponse.md)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## cloud_games_builds_list_game_builds

> crate::models::CloudGamesListGameBuildsResponse cloud_games_builds_list_game_builds(game_id)


Lists game builds for the given game.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**game_id** | **uuid::Uuid** |  | [required] |

### Return type

[**crate::models::CloudGamesListGameBuildsResponse**](CloudGamesListGameBuildsResponse.md)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

