# \CloudGamesNamespacesAnalyticsApi

All URIs are relative to *https://api.rivet.gg*

Method | HTTP request | Description
------------- | ------------- | -------------
[**cloud_games_namespaces_analytics_get_analytics_matchmaker_live**](CloudGamesNamespacesAnalyticsApi.md#cloud_games_namespaces_analytics_get_analytics_matchmaker_live) | **GET** /cloud/games/{game_id}/namespaces/{namespace_id}/analytics/matchmaker/live | 



## cloud_games_namespaces_analytics_get_analytics_matchmaker_live

> crate::models::CloudGamesNamespacesGetAnalyticsMatchmakerLiveResponse cloud_games_namespaces_analytics_get_analytics_matchmaker_live(game_id, namespace_id)


Returns live information about all active lobbies for a given namespace.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**game_id** | **uuid::Uuid** |  | [required] |
**namespace_id** | **uuid::Uuid** |  | [required] |

### Return type

[**crate::models::CloudGamesNamespacesGetAnalyticsMatchmakerLiveResponse**](CloudGamesNamespacesGetAnalyticsMatchmakerLiveResponse.md)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

