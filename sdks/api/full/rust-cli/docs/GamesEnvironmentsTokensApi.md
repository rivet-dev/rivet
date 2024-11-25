# \GamesEnvironmentsTokensApi

All URIs are relative to *https://api.rivet.gg*

Method | HTTP request | Description
------------- | ------------- | -------------
[**games_environments_tokens_create_service_token**](GamesEnvironmentsTokensApi.md#games_environments_tokens_create_service_token) | **POST** /games/{game_id}/environments/{environment_id}/tokens/service | 



## games_environments_tokens_create_service_token

> crate::models::GamesEnvironmentsCreateServiceTokenResponse games_environments_tokens_create_service_token(game_id, environment_id)


Creates a new environment service token.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**game_id** | **uuid::Uuid** |  | [required] |
**environment_id** | **uuid::Uuid** |  | [required] |

### Return type

[**crate::models::GamesEnvironmentsCreateServiceTokenResponse**](GamesEnvironmentsCreateServiceTokenResponse.md)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

