# \RunnerConfigsUpsertApi

All URIs are relative to *http://localhost*

Method | HTTP request | Description
------------- | ------------- | -------------
[**runner_configs_upsert**](RunnerConfigsUpsertApi.md#runner_configs_upsert) | **PUT** /runner-configs/{runner_name} | 



## runner_configs_upsert

> serde_json::Value runner_configs_upsert(runner_name, namespace, runner_configs_upsert_request_body)


### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**runner_name** | **String** |  | [required] |
**namespace** | **String** |  | [required] |
**runner_configs_upsert_request_body** | [**RunnerConfigsUpsertRequestBody**](RunnerConfigsUpsertRequestBody.md) |  | [required] |

### Return type

[**serde_json::Value**](serde_json::Value.md)

### Authorization

[bearer_auth](../README.md#bearer_auth)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

