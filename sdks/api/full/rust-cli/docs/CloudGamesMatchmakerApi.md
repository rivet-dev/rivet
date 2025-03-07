# \CloudGamesMatchmakerApi

All URIs are relative to *https://api.rivet.gg*

Method | HTTP request | Description
------------- | ------------- | -------------
[**cloud_games_matchmaker_delete_matchmaker_lobby**](CloudGamesMatchmakerApi.md#cloud_games_matchmaker_delete_matchmaker_lobby) | **DELETE** /cloud/games/{game_id}/matchmaker/lobbies/{lobby_id} | 
[**cloud_games_matchmaker_export_lobby_logs**](CloudGamesMatchmakerApi.md#cloud_games_matchmaker_export_lobby_logs) | **POST** /cloud/games/{game_id}/matchmaker/lobbies/{lobby_id}/logs/export | 
[**cloud_games_matchmaker_export_matchmaker_lobby_history**](CloudGamesMatchmakerApi.md#cloud_games_matchmaker_export_matchmaker_lobby_history) | **POST** /cloud/games/{game_id}/matchmaker/lobbies/export-history | 
[**cloud_games_matchmaker_get_lobby_logs**](CloudGamesMatchmakerApi.md#cloud_games_matchmaker_get_lobby_logs) | **GET** /cloud/games/{game_id}/matchmaker/lobbies/{lobby_id}/logs | 



## cloud_games_matchmaker_delete_matchmaker_lobby

> crate::models::CloudGamesDeleteMatchmakerLobbyResponse cloud_games_matchmaker_delete_matchmaker_lobby(game_id, lobby_id)


Deletes a matchmaker lobby, stopping it immediately.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**game_id** | **uuid::Uuid** |  | [required] |
**lobby_id** | **uuid::Uuid** |  | [required] |

### Return type

[**crate::models::CloudGamesDeleteMatchmakerLobbyResponse**](CloudGamesDeleteMatchmakerLobbyResponse.md)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## cloud_games_matchmaker_export_lobby_logs

> crate::models::CloudGamesExportLobbyLogsResponse cloud_games_matchmaker_export_lobby_logs(game_id, lobby_id, cloud_games_export_lobby_logs_request)


Generates a download URL for logs.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**game_id** | **uuid::Uuid** |  | [required] |
**lobby_id** | **uuid::Uuid** |  | [required] |
**cloud_games_export_lobby_logs_request** | [**CloudGamesExportLobbyLogsRequest**](CloudGamesExportLobbyLogsRequest.md) |  | [required] |

### Return type

[**crate::models::CloudGamesExportLobbyLogsResponse**](CloudGamesExportLobbyLogsResponse.md)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## cloud_games_matchmaker_export_matchmaker_lobby_history

> crate::models::CloudGamesExportMatchmakerLobbyHistoryResponse cloud_games_matchmaker_export_matchmaker_lobby_history(game_id, cloud_games_export_matchmaker_lobby_history_request)


Exports lobby history over a given query time span.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**game_id** | **uuid::Uuid** |  | [required] |
**cloud_games_export_matchmaker_lobby_history_request** | [**CloudGamesExportMatchmakerLobbyHistoryRequest**](CloudGamesExportMatchmakerLobbyHistoryRequest.md) |  | [required] |

### Return type

[**crate::models::CloudGamesExportMatchmakerLobbyHistoryResponse**](CloudGamesExportMatchmakerLobbyHistoryResponse.md)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## cloud_games_matchmaker_get_lobby_logs

> crate::models::CloudGamesGetLobbyLogsResponse cloud_games_matchmaker_get_lobby_logs(game_id, lobby_id, stream, watch_index)


Returns the logs for a given lobby.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**game_id** | **uuid::Uuid** |  | [required] |
**lobby_id** | **uuid::Uuid** |  | [required] |
**stream** | [**CloudGamesLogStream**](.md) |  | [required] |
**watch_index** | Option<**String**> | A query parameter denoting the requests watch index. |  |

### Return type

[**crate::models::CloudGamesGetLobbyLogsResponse**](CloudGamesGetLobbyLogsResponse.md)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

