# \RunnerConfigsRefreshMetadataApi

All URIs are relative to *http://localhost*

Method | HTTP request | Description
------------- | ------------- | -------------
[**runner_configs_refresh_metadata**](RunnerConfigsRefreshMetadataApi.md#runner_configs_refresh_metadata) | **POST** /runner-configs/{runner_name}/refresh-metadata | 



## runner_configs_refresh_metadata

> serde_json::Value runner_configs_refresh_metadata(runner_name, namespace, body)


### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**runner_name** | **String** |  | [required] |
**namespace** | **String** |  | [required] |
**body** | **serde_json::Value** |  | [required] |

### Return type

[**serde_json::Value**](serde_json::Value.md)

### Authorization

[bearer_auth](../README.md#bearer_auth)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

