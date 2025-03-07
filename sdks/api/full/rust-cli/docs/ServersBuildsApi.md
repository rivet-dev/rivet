# \ServersBuildsApi

All URIs are relative to *https://api.rivet.gg*

Method | HTTP request | Description
------------- | ------------- | -------------
[**servers_builds_complete**](ServersBuildsApi.md#servers_builds_complete) | **POST** /games/{game_id}/environments/{environment_id}/builds/{build_id}/complete | 
[**servers_builds_get**](ServersBuildsApi.md#servers_builds_get) | **GET** /games/{game_id}/environments/{environment_id}/builds/{build_id} | 
[**servers_builds_list**](ServersBuildsApi.md#servers_builds_list) | **GET** /games/{game_id}/environments/{environment_id}/builds | 
[**servers_builds_patch_tags**](ServersBuildsApi.md#servers_builds_patch_tags) | **PATCH** /games/{game_id}/environments/{environment_id}/builds/{build_id}/tags | 
[**servers_builds_prepare**](ServersBuildsApi.md#servers_builds_prepare) | **POST** /games/{game_id}/environments/{environment_id}/builds/prepare | 



## servers_builds_complete

> servers_builds_complete(game_id, environment_id, build_id)


Marks an upload as complete.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**game_id** | **uuid::Uuid** |  | [required] |
**environment_id** | **uuid::Uuid** |  | [required] |
**build_id** | **uuid::Uuid** |  | [required] |

### Return type

 (empty response body)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## servers_builds_get

> crate::models::ServersGetBuildResponse servers_builds_get(game_id, environment_id, build_id, tags_json)


Lists all builds of the game associated with the token used. Can be filtered by tags in the query string.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**game_id** | **uuid::Uuid** |  | [required] |
**environment_id** | **uuid::Uuid** |  | [required] |
**build_id** | **uuid::Uuid** |  | [required] |
**tags_json** | Option<**String**> |  |  |

### Return type

[**crate::models::ServersGetBuildResponse**](ServersGetBuildResponse.md)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## servers_builds_list

> crate::models::ServersListBuildsResponse servers_builds_list(game_id, environment_id, tags_json)


Lists all builds of the game associated with the token used. Can be filtered by tags in the query string.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**game_id** | **uuid::Uuid** |  | [required] |
**environment_id** | **uuid::Uuid** |  | [required] |
**tags_json** | Option<**String**> |  |  |

### Return type

[**crate::models::ServersListBuildsResponse**](ServersListBuildsResponse.md)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## servers_builds_patch_tags

> serde_json::Value servers_builds_patch_tags(game_id, environment_id, build_id, servers_patch_build_tags_request)


### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**game_id** | **uuid::Uuid** |  | [required] |
**environment_id** | **uuid::Uuid** |  | [required] |
**build_id** | **uuid::Uuid** |  | [required] |
**servers_patch_build_tags_request** | [**ServersPatchBuildTagsRequest**](ServersPatchBuildTagsRequest.md) |  | [required] |

### Return type

[**serde_json::Value**](serde_json::Value.md)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## servers_builds_prepare

> crate::models::ServersCreateBuildResponse servers_builds_prepare(game_id, environment_id, servers_create_build_request)


Creates a new game build for the given game.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**game_id** | **uuid::Uuid** |  | [required] |
**environment_id** | **uuid::Uuid** |  | [required] |
**servers_create_build_request** | [**ServersCreateBuildRequest**](ServersCreateBuildRequest.md) |  | [required] |

### Return type

[**crate::models::ServersCreateBuildResponse**](ServersCreateBuildResponse.md)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

