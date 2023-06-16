# \CloudGamesVersionsApi

All URIs are relative to *http://localhost*

Method | HTTP request | Description
------------- | ------------- | -------------
[**cloud_games_versions_create_game_version**](CloudGamesVersionsApi.md#cloud_games_versions_create_game_version) | **POST** /games/{game_id}/versions | 
[**cloud_games_versions_get_game_version_by_id**](CloudGamesVersionsApi.md#cloud_games_versions_get_game_version_by_id) | **GET** /games/{game_id}/versions/{version_id} | 
[**cloud_games_versions_validate_game_version**](CloudGamesVersionsApi.md#cloud_games_versions_validate_game_version) | **POST** /games/{game_id}/versions/validate | 



## cloud_games_versions_create_game_version

> crate::models::CloudGamesCreateGameVersionResponse cloud_games_versions_create_game_version(game_id, cloud_games_create_game_version_request)


Creates a new game version.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**game_id** | **uuid::Uuid** |  | [required] |
**cloud_games_create_game_version_request** | [**CloudGamesCreateGameVersionRequest**](CloudGamesCreateGameVersionRequest.md) |  | [required] |

### Return type

[**crate::models::CloudGamesCreateGameVersionResponse**](CloudGamesCreateGameVersionResponse.md)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## cloud_games_versions_get_game_version_by_id

> crate::models::CloudGamesGetGameVersionByIdResponse cloud_games_versions_get_game_version_by_id(game_id, version_id)


Returns a game version by its version ID.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**game_id** | **uuid::Uuid** |  | [required] |
**version_id** | **uuid::Uuid** |  | [required] |

### Return type

[**crate::models::CloudGamesGetGameVersionByIdResponse**](CloudGamesGetGameVersionByIdResponse.md)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## cloud_games_versions_validate_game_version

> crate::models::CloudGamesValidateGameVersionResponse cloud_games_versions_validate_game_version(game_id, cloud_games_validate_game_version_request)


Validates information used to create a new game version.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**game_id** | **uuid::Uuid** |  | [required] |
**cloud_games_validate_game_version_request** | [**CloudGamesValidateGameVersionRequest**](CloudGamesValidateGameVersionRequest.md) |  | [required] |

### Return type

[**crate::models::CloudGamesValidateGameVersionResponse**](CloudGamesValidateGameVersionResponse.md)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

