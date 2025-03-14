# \BuildsApi

All URIs are relative to *https://api.rivet.gg*

Method | HTTP request | Description
------------- | ------------- | -------------
[**builds_complete**](BuildsApi.md#builds_complete) | **POST** /builds/{build}/complete | 
[**builds_get**](BuildsApi.md#builds_get) | **GET** /builds/{build} | 
[**builds_list**](BuildsApi.md#builds_list) | **GET** /builds | 
[**builds_patch_tags**](BuildsApi.md#builds_patch_tags) | **PATCH** /builds/{build}/tags | 
[**builds_prepare**](BuildsApi.md#builds_prepare) | **POST** /builds/prepare | 



## builds_complete

> builds_complete(build, project, environment)


Marks an upload as complete.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**build** | **uuid::Uuid** |  | [required] |
**project** | Option<**String**> |  |  |
**environment** | Option<**String**> |  |  |

### Return type

 (empty response body)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## builds_get

> crate::models::BuildsGetBuildResponse builds_get(build, project, environment)


Get a build.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**build** | **uuid::Uuid** |  | [required] |
**project** | Option<**String**> |  |  |
**environment** | Option<**String**> |  |  |

### Return type

[**crate::models::BuildsGetBuildResponse**](BuildsGetBuildResponse.md)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## builds_list

> crate::models::BuildsListBuildsResponse builds_list(project, environment, tags_json)


Lists all builds of the project associated with the token used. Can be filtered by tags in the query string.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**project** | Option<**String**> |  |  |
**environment** | Option<**String**> |  |  |
**tags_json** | Option<**String**> |  |  |

### Return type

[**crate::models::BuildsListBuildsResponse**](BuildsListBuildsResponse.md)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## builds_patch_tags

> serde_json::Value builds_patch_tags(build, builds_patch_build_tags_request, project, environment)


### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**build** | **uuid::Uuid** |  | [required] |
**builds_patch_build_tags_request** | [**BuildsPatchBuildTagsRequest**](BuildsPatchBuildTagsRequest.md) |  | [required] |
**project** | Option<**String**> |  |  |
**environment** | Option<**String**> |  |  |

### Return type

[**serde_json::Value**](serde_json::Value.md)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## builds_prepare

> crate::models::BuildsPrepareBuildResponse builds_prepare(builds_prepare_build_request, project, environment)


Creates a new project build for the given project.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**builds_prepare_build_request** | [**BuildsPrepareBuildRequest**](BuildsPrepareBuildRequest.md) |  | [required] |
**project** | Option<**String**> |  |  |
**environment** | Option<**String**> |  |  |

### Return type

[**crate::models::BuildsPrepareBuildResponse**](BuildsPrepareBuildResponse.md)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

