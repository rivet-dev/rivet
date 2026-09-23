# \RunnerConfigsListApi

All URIs are relative to *http://localhost*

Method | HTTP request | Description
------------- | ------------- | -------------
[**runner_configs_list**](RunnerConfigsListApi.md#runner_configs_list) | **GET** /runner-configs | 



## runner_configs_list

> models::RunnerConfigsListResponse runner_configs_list(namespace, limit, cursor, variant, runner_names, runner_name)


### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**namespace** | **String** |  | [required] |
**limit** | Option<**i32**> |  |  |
**cursor** | Option<**String**> |  |  |
**variant** | Option<[**RunnerConfigVariant**](.md)> |  |  |
**runner_names** | Option<**String**> | Deprecated. |  |
**runner_name** | Option<[**Vec<String>**](String.md)> |  |  |

### Return type

[**models::RunnerConfigsListResponse**](RunnerConfigsListResponse.md)

### Authorization

[bearer_auth](../README.md#bearer_auth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

