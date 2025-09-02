# \ActorsCreateApi

All URIs are relative to *http://localhost*

Method | HTTP request | Description
------------- | ------------- | -------------
[**actors_create**](ActorsCreateApi.md#actors_create) | **POST** /actors | ## Datacenter Round Trips



## actors_create

> models::ActorsCreateResponse actors_create(namespace, actors_create_request, datacenter)
## Datacenter Round Trips

**If actor is created in the current datacenter:**  2 round trips: - namespace::ops::resolve_for_name_global - [pegboard::workflows::actor] Create actor workflow (includes Epoxy key allocation)  **If actor is created in a different datacenter:**  3 round trips: - namespace::ops::resolve_for_name_global - POST /actors to remote datacenter - [pegboard::workflows::actor] Create actor workflow (includes Epoxy key allocation)  actor::get will always be in the same datacenter.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**namespace** | **String** |  | [required] |
**actors_create_request** | [**ActorsCreateRequest**](ActorsCreateRequest.md) |  | [required] |
**datacenter** | Option<**String**> |  |  |

### Return type

[**models::ActorsCreateResponse**](ActorsCreateResponse.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

