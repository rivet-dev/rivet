# \RunnerConfigsServerlessHealthCheckApi

All URIs are relative to *http://localhost*

Method | HTTP request | Description
------------- | ------------- | -------------
[**runner_configs_serverless_health_check**](RunnerConfigsServerlessHealthCheckApi.md#runner_configs_serverless_health_check) | **POST** /runner-configs/serverless-health-check | 



## runner_configs_serverless_health_check

> models::RunnerConfigsServerlessHealthCheckResponse runner_configs_serverless_health_check(namespace, runner_configs_serverless_health_check_request)


### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**namespace** | **String** |  | [required] |
**runner_configs_serverless_health_check_request** | [**RunnerConfigsServerlessHealthCheckRequest**](RunnerConfigsServerlessHealthCheckRequest.md) |  | [required] |

### Return type

[**models::RunnerConfigsServerlessHealthCheckResponse**](RunnerConfigsServerlessHealthCheckResponse.md)

### Authorization

[bearer_auth](../README.md#bearer_auth)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

