# \ActorsGetByIdApi

All URIs are relative to *http://localhost*

Method | HTTP request | Description
------------- | ------------- | -------------
[**actors_get_by_id**](ActorsGetByIdApi.md#actors_get_by_id) | **GET** /actors/by-id | ## Datacenter Round Trips



## actors_get_by_id

> models::ActorsGetByIdResponse actors_get_by_id(namespace, name, key)
## Datacenter Round Trips

1 round trip: - namespace::ops::resolve_for_name_global  This does not require another round trip since we use stale consistency for the get_id_for_key.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**namespace** | **String** |  | [required] |
**name** | **String** |  | [required] |
**key** | **String** |  | [required] |

### Return type

[**models::ActorsGetByIdResponse**](ActorsGetByIdResponse.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

