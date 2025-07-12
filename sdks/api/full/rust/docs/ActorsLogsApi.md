# \ActorsLogsApi

All URIs are relative to *https://api.rivet.gg*

Method | HTTP request | Description
------------- | ------------- | -------------
[**actors_logs_export**](ActorsLogsApi.md#actors_logs_export) | **POST** /actors/logs/export | 
[**actors_logs_get**](ActorsLogsApi.md#actors_logs_get) | **GET** /actors/logs | 



## actors_logs_export

> crate::models::ActorsExportActorLogsResponse actors_logs_export(actors_logs_export_request)


Exports logs for the given actors to an S3 bucket and returns a presigned URL to download.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**actors_logs_export_request** | [**ActorsLogsExportRequest**](ActorsLogsExportRequest.md) |  | [required] |

### Return type

[**crate::models::ActorsExportActorLogsResponse**](ActorsExportActorLogsResponse.md)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## actors_logs_get

> crate::models::ActorsGetActorLogsResponse actors_logs_get(project, environment, query_json, watch_index)


Returns the logs for a given actor.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**project** | Option<**String**> |  |  |
**environment** | Option<**String**> |  |  |
**query_json** | Option<**String**> | JSON-encoded query expression for filtering logs |  |
**watch_index** | Option<**String**> | A query parameter denoting the requests watch index. |  |

### Return type

[**crate::models::ActorsGetActorLogsResponse**](ActorsGetActorLogsResponse.md)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

