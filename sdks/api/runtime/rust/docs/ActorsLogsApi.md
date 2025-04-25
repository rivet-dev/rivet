# \ActorsLogsApi

All URIs are relative to *https://api.rivet.gg*

Method | HTTP request | Description
------------- | ------------- | -------------
[**actors_logs_get**](ActorsLogsApi.md#actors_logs_get) | **GET** /actors/logs | 



## actors_logs_get

> crate::models::ActorsGetActorLogsResponse actors_logs_get(stream, actor_ids_json, project, environment, search_text, search_case_sensitive, search_enable_regex, watch_index)


Returns the logs for a given actor.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**stream** | [**ActorsQueryLogStream**](.md) |  | [required] |
**actor_ids_json** | **String** |  | [required] |
**project** | Option<**String**> |  |  |
**environment** | Option<**String**> |  |  |
**search_text** | Option<**String**> |  |  |
**search_case_sensitive** | Option<**bool**> |  |  |
**search_enable_regex** | Option<**bool**> |  |  |
**watch_index** | Option<**String**> | A query parameter denoting the requests watch index. |  |

### Return type

[**crate::models::ActorsGetActorLogsResponse**](ActorsGetActorLogsResponse.md)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

