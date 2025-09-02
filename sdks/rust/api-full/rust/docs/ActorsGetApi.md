# \ActorsGetApi

All URIs are relative to *http://localhost*

Method | HTTP request | Description
------------- | ------------- | -------------
[**actors_get**](ActorsGetApi.md#actors_get) | **GET** /actors/{actor_id} | ## Datacenter Round Trips



## actors_get

> models::ActorsGetResponse actors_get(actor_id, namespace)
## Datacenter Round Trips

2 round trip: - GET /actors/{} - [api-peer] namespace::ops::resolve_for_name_global

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**actor_id** | **String** |  | [required] |
**namespace** | Option<**String**> |  |  |

### Return type

[**models::ActorsGetResponse**](ActorsGetResponse.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

