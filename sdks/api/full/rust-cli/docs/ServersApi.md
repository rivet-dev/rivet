# \ServersApi

All URIs are relative to *https://api.rivet.gg*

Method | HTTP request | Description
------------- | ------------- | -------------
[**servers_create**](ServersApi.md#servers_create) | **POST** /games/{game_id}/environments/{environment_id}/servers | 
[**servers_destroy**](ServersApi.md#servers_destroy) | **DELETE** /games/{game_id}/environments/{environment_id}/servers/{server_id} | 
[**servers_get**](ServersApi.md#servers_get) | **GET** /games/{game_id}/environments/{environment_id}/servers/{server_id} | 
[**servers_list**](ServersApi.md#servers_list) | **GET** /games/{game_id}/environments/{environment_id}/servers | 



## servers_create

> crate::models::ServersCreateServerResponse servers_create(game_id, environment_id, servers_create_server_request)


Create a new dynamic server.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**game_id** | **uuid::Uuid** |  | [required] |
**environment_id** | **uuid::Uuid** |  | [required] |
**servers_create_server_request** | [**ServersCreateServerRequest**](ServersCreateServerRequest.md) |  | [required] |

### Return type

[**crate::models::ServersCreateServerResponse**](ServersCreateServerResponse.md)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## servers_destroy

> serde_json::Value servers_destroy(game_id, environment_id, server_id, override_kill_timeout)


Destroy a dynamic server.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**game_id** | **uuid::Uuid** |  | [required] |
**environment_id** | **uuid::Uuid** |  | [required] |
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


## servers_get

> crate::models::ServersGetServerResponse servers_get(game_id, environment_id, server_id)


Gets a dynamic server.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**game_id** | **uuid::Uuid** |  | [required] |
**environment_id** | **uuid::Uuid** |  | [required] |
**server_id** | **uuid::Uuid** | The id of the server to destroy | [required] |

### Return type

[**crate::models::ServersGetServerResponse**](ServersGetServerResponse.md)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## servers_list

> crate::models::ServersListServersResponse servers_list(game_id, environment_id, tags_json, include_destroyed, cursor)


Lists all servers associated with the token used. Can be filtered by tags in the query string.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**game_id** | **uuid::Uuid** |  | [required] |
**environment_id** | **uuid::Uuid** |  | [required] |
**tags_json** | Option<**String**> |  |  |
**include_destroyed** | Option<**bool**> |  |  |
**cursor** | Option<**uuid::Uuid**> |  |  |

### Return type

[**crate::models::ServersListServersResponse**](ServersListServersResponse.md)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

