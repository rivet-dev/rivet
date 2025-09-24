# \RunnerConfigsApi

All URIs are relative to *http://localhost*

Method | HTTP request | Description
------------- | ------------- | -------------
[**runner_configs_delete**](RunnerConfigsApi.md#runner_configs_delete) | **DELETE** /runner-configs/{runner_name} | 
[**runner_configs_list**](RunnerConfigsApi.md#runner_configs_list) | **GET** /runner-configs | 
[**runner_configs_upsert**](RunnerConfigsApi.md#runner_configs_upsert) | **PUT** /runner-configs/{runner_name} | 



## runner_configs_delete

> serde_json::Value runner_configs_delete(runner_name, namespace)


### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**runner_name** | **String** |  | [required] |
**namespace** | **String** |  | [required] |

### Return type

[**serde_json::Value**](serde_json::Value.md)

### Authorization

[bearer_auth](../README.md#bearer_auth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## runner_configs_list

> models::RunnerConfigsListResponse runner_configs_list(namespace, limit, cursor, variant, runner_names)


### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**namespace** | **String** |  | [required] |
**limit** | Option<**i32**> |  |  |
**cursor** | Option<**String**> |  |  |
**variant** | Option<[**RunnerConfigVariant**](.md)> |  |  |
**runner_names** | Option<**String**> |  |  |

### Return type

[**models::RunnerConfigsListResponse**](RunnerConfigsListResponse.md)

### Authorization

[bearer_auth](../README.md#bearer_auth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## runner_configs_upsert

> serde_json::Value runner_configs_upsert(runner_name, namespace, runner_configs_upsert_request)


### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**runner_name** | **String** |  | [required] |
**namespace** | **String** |  | [required] |
**runner_configs_upsert_request** | [**RunnerConfigsUpsertRequest**](RunnerConfigsUpsertRequest.md) |  | [required] |

### Return type

[**serde_json::Value**](serde_json::Value.md)

### Authorization

[bearer_auth](../README.md#bearer_auth)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

