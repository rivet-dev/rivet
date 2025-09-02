# \ActorsListNamesApi

All URIs are relative to *http://localhost*

Method | HTTP request | Description
------------- | ------------- | -------------
[**actors_list_names**](ActorsListNamesApi.md#actors_list_names) | **GET** /actors/names | ## Datacenter Round Trips



## actors_list_names

> models::ActorsListNamesResponse actors_list_names(namespace, limit, cursor)
## Datacenter Round Trips

2 round trips: - GET /actors/names (fanout) - [api-peer] namespace::ops::resolve_for_name_global

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**namespace** | **String** |  | [required] |
**limit** | Option<**i32**> |  |  |
**cursor** | Option<**String**> |  |  |

### Return type

[**models::ActorsListNamesResponse**](ActorsListNamesResponse.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

