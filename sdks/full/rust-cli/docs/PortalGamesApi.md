# \PortalGamesApi

All URIs are relative to *https://api.rivet.gg*

Method | HTTP request | Description
------------- | ------------- | -------------
[**portal_games_get_game_profile**](PortalGamesApi.md#portal_games_get_game_profile) | **GET** /portal/games/{game_name_id}/profile | 



## portal_games_get_game_profile

> crate::models::PortalGetGameProfileResponse portal_games_get_game_profile(game_name_id, watch_index)


Returns a game profile.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**game_name_id** | **String** | A human readable short identifier used to references resources. Different than a `rivet.common#Uuid` because this is intended to be human readable. Different than `rivet.common#DisplayName` because this should not include special characters and be short. | [required] |
**watch_index** | Option<**String**> | A query parameter denoting the requests watch index. |  |

### Return type

[**crate::models::PortalGetGameProfileResponse**](PortalGetGameProfileResponse.md)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

