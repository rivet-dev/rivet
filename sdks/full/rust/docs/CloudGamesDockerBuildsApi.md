# \CloudGamesDockerBuildsApi

All URIs are relative to *https://api.rivet.gg*

Method | HTTP request | Description
------------- | ------------- | -------------
[**cloud_games_docker_builds_create_game_build**](CloudGamesDockerBuildsApi.md#cloud_games_docker_builds_create_game_build) | **POST** /cloud/games/{game_id}/docker/builds | 
[**cloud_games_docker_builds_list_game_builds**](CloudGamesDockerBuildsApi.md#cloud_games_docker_builds_list_game_builds) | **GET** /cloud/games/{game_id}/docker/builds | 



## cloud_games_docker_builds_create_game_build

> crate::models::CloudGamesDockerCreateGameBuildResponse cloud_games_docker_builds_create_game_build(game_id, cloud_games_docker_create_game_build_request)


Creates a new game build for the given game.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**game_id** | **uuid::Uuid** |  | [required] |
**cloud_games_docker_create_game_build_request** | [**CloudGamesDockerCreateGameBuildRequest**](CloudGamesDockerCreateGameBuildRequest.md) |  | [required] |

### Return type

[**crate::models::CloudGamesDockerCreateGameBuildResponse**](CloudGamesDockerCreateGameBuildResponse.md)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## cloud_games_docker_builds_list_game_builds

> crate::models::CloudGamesDockerListGameBuildsResponse cloud_games_docker_builds_list_game_builds(game_id)


Lists game builds for the given game.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**game_id** | **uuid::Uuid** |  | [required] |

### Return type

[**crate::models::CloudGamesDockerListGameBuildsResponse**](CloudGamesDockerListGameBuildsResponse.md)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

