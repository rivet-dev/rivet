# \ActorsKvGetApi

All URIs are relative to *http://localhost*

Method | HTTP request | Description
------------- | ------------- | -------------
[**actors_kv_get**](ActorsKvGetApi.md#actors_kv_get) | **GET** /actors/{actor_id}/kv/keys/{key} | 



## actors_kv_get

> models::ActorsKvGetResponse actors_kv_get(actor_id, key)


### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**actor_id** | **String** |  | [required] |
**key** | **String** |  | [required] |

### Return type

[**models::ActorsKvGetResponse**](ActorsKvGetResponse.md)

### Authorization

[bearer_auth](../README.md#bearer_auth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

