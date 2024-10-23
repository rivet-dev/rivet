# \ActorBuildsApi

All URIs are relative to *https://api.rivet.gg*

Method | HTTP request | Description
------------- | ------------- | -------------
[**actor_builds_complete**](ActorBuildsApi.md#actor_builds_complete) | **POST** /games/{game_id}/environments/{environment_id}/builds/{build_id}/complete | 
[**actor_builds_get**](ActorBuildsApi.md#actor_builds_get) | **GET** /games/{game_id}/environments/{environment_id}/builds/{build_id} | 
[**actor_builds_list**](ActorBuildsApi.md#actor_builds_list) | **GET** /games/{game_id}/environments/{environment_id}/builds | 
[**actor_builds_patch_tags**](ActorBuildsApi.md#actor_builds_patch_tags) | **PATCH** /games/{game_id}/environments/{environment_id}/builds/{build_id}/tags | 
[**actor_builds_prepare**](ActorBuildsApi.md#actor_builds_prepare) | **POST** /games/{game_id}/environments/{environment_id}/builds/prepare | 



## actor_builds_complete

> actor_builds_complete(game_id, environment_id, build_id)


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


## actor_builds_get

> crate::models::ActorGetBuildResponse actor_builds_get(game_id, environment_id, build_id, tags_json)


Lists all builds of the game associated with the token used. Can be filtered by tags in the query string.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**game_id** | **uuid::Uuid** |  | [required] |
**environment_id** | **uuid::Uuid** |  | [required] |
**build_id** | **uuid::Uuid** |  | [required] |
**tags_json** | Option<**String**> |  |  |

### Return type

[**crate::models::ActorGetBuildResponse**](ActorGetBuildResponse.md)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## actor_builds_list

> crate::models::ActorListBuildsResponse actor_builds_list(game_id, environment_id, tags_json)


Lists all builds of the game associated with the token used. Can be filtered by tags in the query string.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**game_id** | **uuid::Uuid** |  | [required] |
**environment_id** | **uuid::Uuid** |  | [required] |
**tags_json** | Option<**String**> |  |  |

### Return type

[**crate::models::ActorListBuildsResponse**](ActorListBuildsResponse.md)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## actor_builds_patch_tags

> serde_json::Value actor_builds_patch_tags(game_id, environment_id, build_id, actor_patch_build_tags_request)


### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**game_id** | **uuid::Uuid** |  | [required] |
**environment_id** | **uuid::Uuid** |  | [required] |
**build_id** | **uuid::Uuid** |  | [required] |
**actor_patch_build_tags_request** | [**ActorPatchBuildTagsRequest**](ActorPatchBuildTagsRequest.md) |  | [required] |

### Return type

[**serde_json::Value**](serde_json::Value.md)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## actor_builds_prepare

> crate::models::ActorCreateBuildResponse actor_builds_prepare(game_id, environment_id, actor_create_build_request)


Creates a new game build for the given game.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**game_id** | **uuid::Uuid** |  | [required] |
**environment_id** | **uuid::Uuid** |  | [required] |
**actor_create_build_request** | [**ActorCreateBuildRequest**](ActorCreateBuildRequest.md) |  | [required] |

### Return type

[**crate::models::ActorCreateBuildResponse**](ActorCreateBuildResponse.md)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

