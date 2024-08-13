# \GamesServersApi

All URIs are relative to *https://api.rivet.gg*

Method | HTTP request | Description
------------- | ------------- | -------------
[**games_servers_create**](GamesServersApi.md#games_servers_create) | **POST** /games/{game_id}/servers | 
[**games_servers_destroy**](GamesServersApi.md#games_servers_destroy) | **DELETE** /games/{game_id}/servers/{server_id} | 
[**games_servers_get**](GamesServersApi.md#games_servers_get) | **GET** /games/{game_id}/servers/{server_id} | 
[**games_servers_list**](GamesServersApi.md#games_servers_list) | **GET** /games/{game_id}/servers | 



## games_servers_create

> crate::models::GamesServersCreateServerResponse games_servers_create(game_id, games_servers_create_server_request)


Create a new dynamic server.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**game_id** | **uuid::Uuid** |  | [required] |
**games_servers_create_server_request** | [**GamesServersCreateServerRequest**](GamesServersCreateServerRequest.md) |  | [required] |

### Return type

[**crate::models::GamesServersCreateServerResponse**](GamesServersCreateServerResponse.md)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## games_servers_destroy

> serde_json::Value games_servers_destroy(game_id, server_id, override_kill_timeout)


Destroy a dynamic server.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**game_id** | **uuid::Uuid** |  | [required] |
**server_id** | **uuid::Uuid** | The id of the server to destroy | [required] |
**override_kill_timeout** | Option<**i64**> | The duration to wait for in milliseconds before killing the server. This should be used to override the default kill timeout if a faster time is needed, say for ignoring a graceful shutdown. |  |

### Return type

[**serde_json::Value**](serde_json::Value.md)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## games_servers_get

> crate::models::GamesServersGetServerResponse games_servers_get(game_id, server_id)


Gets a dynamic server.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**game_id** | **uuid::Uuid** |  | [required] |
**server_id** | **uuid::Uuid** | The id of the server to destroy | [required] |

### Return type

[**crate::models::GamesServersGetServerResponse**](GamesServersGetServerResponse.md)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## games_servers_list

> crate::models::GamesServersListServersResponse games_servers_list(game_id, tags, game_id2)


Lists all servers associated with the token used. Can be filtered by tags in the query string.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**game_id** | **uuid::Uuid** |  | [required] |
**tags** | Option<**String**> |  |  |
**game_id2** | Option<**uuid::Uuid**> |  |  |

### Return type

[**crate::models::GamesServersListServersResponse**](GamesServersListServersResponse.md)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

