# \ActorLogsApi

All URIs are relative to *https://api.rivet.gg*

Method | HTTP request | Description
------------- | ------------- | -------------
[**actor_logs_get**](ActorLogsApi.md#actor_logs_get) | **GET** /actors/{actor}/logs | 



## actor_logs_get

> crate::models::ActorGetActorLogsResponse actor_logs_get(actor, stream, project, environment, watch_index)


Returns the logs for a given actor.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**actor** | **uuid::Uuid** |  | [required] |
**stream** | [**ActorLogStream**](.md) |  | [required] |
**project** | Option<**String**> |  |  |
**environment** | Option<**String**> |  |  |
**watch_index** | Option<**String**> | A query parameter denoting the requests watch index. |  |

### Return type

[**crate::models::ActorGetActorLogsResponse**](ActorGetActorLogsResponse.md)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

