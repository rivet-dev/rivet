# \CloudLogsApi

All URIs are relative to *http://localhost*

Method | HTTP request | Description
------------- | ------------- | -------------
[**cloud_logs_get_ray_perf_logs**](CloudLogsApi.md#cloud_logs_get_ray_perf_logs) | **GET** /rays/{ray_id}/perf | 



## cloud_logs_get_ray_perf_logs

> crate::models::CloudGetRayPerfLogsResponse cloud_logs_get_ray_perf_logs(ray_id)


Returns performance information about a Rivet Ray.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**ray_id** | **uuid::Uuid** |  | [required] |

### Return type

[**crate::models::CloudGetRayPerfLogsResponse**](CloudGetRayPerfLogsResponse.md)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

