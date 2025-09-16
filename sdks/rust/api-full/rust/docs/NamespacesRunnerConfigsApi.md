# \NamespacesRunnerConfigsApi

All URIs are relative to *http://localhost*

Method | HTTP request | Description
------------- | ------------- | -------------
[**namespaces_runner_configs_delete**](NamespacesRunnerConfigsApi.md#namespaces_runner_configs_delete) | **DELETE** /namespaces/{namespace_id}/runner-configs/{runner_name} | 
[**namespaces_runner_configs_get**](NamespacesRunnerConfigsApi.md#namespaces_runner_configs_get) | **GET** /namespaces/{namespace_id}/runner-configs/{runner_name} | 
[**namespaces_runner_configs_list**](NamespacesRunnerConfigsApi.md#namespaces_runner_configs_list) | **GET** /namespaces/{namespace_id}/runner-configs | 
[**namespaces_runner_configs_upsert**](NamespacesRunnerConfigsApi.md#namespaces_runner_configs_upsert) | **PUT** /namespaces/{namespace_id}/runner-configs/{runner_name} | 



## namespaces_runner_configs_delete

> serde_json::Value namespaces_runner_configs_delete(namespace_id, runner_name)


### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**namespace_id** | **String** |  | [required] |
**runner_name** | **String** |  | [required] |

### Return type

[**serde_json::Value**](serde_json::Value.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## namespaces_runner_configs_get

> models::NamespacesRunnerConfigsGetResponse namespaces_runner_configs_get(namespace_id, runner_name)


### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**namespace_id** | **String** |  | [required] |
**runner_name** | **String** |  | [required] |

### Return type

[**models::NamespacesRunnerConfigsGetResponse**](NamespacesRunnerConfigsGetResponse.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## namespaces_runner_configs_list

> models::NamespacesRunnerConfigsListResponse namespaces_runner_configs_list(namespace_id, limit, cursor, variant)


### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**namespace_id** | **String** |  | [required] |
**limit** | Option<**i32**> |  |  |
**cursor** | Option<**String**> |  |  |
**variant** | Option<[**NamespacesRunnerConfigVariant**](.md)> |  |  |

### Return type

[**models::NamespacesRunnerConfigsListResponse**](NamespacesRunnerConfigsListResponse.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## namespaces_runner_configs_upsert

> serde_json::Value namespaces_runner_configs_upsert(namespace_id, runner_name, body)


### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**namespace_id** | **String** |  | [required] |
**runner_name** | **String** |  | [required] |
**body** | **models::NamespacesRunnerConfig** |  | [required] |

### Return type

[**serde_json::Value**](serde_json::Value.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

