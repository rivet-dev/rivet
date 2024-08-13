# \GamesServersLogsApi

All URIs are relative to *https://api.rivet.gg*

Method | HTTP request | Description
------------- | ------------- | -------------
[**games_servers_logs_get_server_logs**](GamesServersLogsApi.md#games_servers_logs_get_server_logs) | **GET** /games/{game_id}/servers/{server_id}/logs | 



## games_servers_logs_get_server_logs

> crate::models::GamesServersGetServerLogsResponse games_servers_logs_get_server_logs(game_id, server_id, stream, game_id2, watch_index)


Returns the logs for a given server.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**game_id** | **uuid::Uuid** |  | [required] |
**server_id** | **uuid::Uuid** |  | [required] |
**stream** | [**GamesServersLogStream**](.md) |  | [required] |
**game_id2** | Option<**uuid::Uuid**> |  |  |
**watch_index** | Option<**String**> | A query parameter denoting the requests watch index. |  |

### Return type

[**crate::models::GamesServersGetServerLogsResponse**](GamesServersGetServerLogsResponse.md)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

