# \ActorsDeleteApi

All URIs are relative to *http://localhost*

Method | HTTP request | Description
------------- | ------------- | -------------
[**actors_delete**](ActorsDeleteApi.md#actors_delete) | **DELETE** /actors/{actor_id} | ## Datacenter Round Trips



## actors_delete

> serde_json::Value actors_delete(actor_id, namespace)
## Datacenter Round Trips

2 round trip: - DELETE /actors/{} - [api-peer] namespace::ops::resolve_for_name_global

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**actor_id** | **String** |  | [required] |
**namespace** | Option<**String**> |  |  |

### Return type

[**serde_json::Value**](serde_json::Value.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

