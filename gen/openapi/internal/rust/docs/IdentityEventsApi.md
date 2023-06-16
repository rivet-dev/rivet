# \IdentityEventsApi

All URIs are relative to *http://localhost*

Method | HTTP request | Description
------------- | ------------- | -------------
[**identity_events_watch**](IdentityEventsApi.md#identity_events_watch) | **GET** /events/live | 



## identity_events_watch

> crate::models::IdentityWatchEventsResponse identity_events_watch(watch_index)


Returns all events relative to the current identity.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**watch_index** | Option<**String**> |  |  |

### Return type

[**crate::models::IdentityWatchEventsResponse**](IdentityWatchEventsResponse.md)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

