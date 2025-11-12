# \ActorsListApi

All URIs are relative to *http://localhost*

Method | HTTP request | Description
------------- | ------------- | -------------
[**actors_list**](ActorsListApi.md#actors_list) | **GET** /actors |  ## Datacenter Round Trips



## actors_list

> models::ActorsListResponse actors_list(namespace, name, key, actor_ids, actor_id, include_destroyed, limit, cursor)
 ## Datacenter Round Trips

 **If key is some & `include_destroyed` is false**   2 round trips:  - namespace::ops::resolve_for_name_global  - GET /actors (multiple DCs based on actor IDs)   This path is optimized because we can read the actor IDs fro the key directly from Epoxy with  stale consistency to determine which datacenter the actor lives in. Under most circumstances,  this means we don't need to fan out to all datacenters (like normal list does).   The reason `include_destroyed` has to be false is Epoxy only stores currently active actors. If  `include_destroyed` is true, we show all previous iterations of actors with the same key.   **Otherwise**   2 round trips:  - namespace::ops::resolve_for_name_global  - GET /actors (fanout)   ## Optimized Alternative Routes

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**namespace** | **String** |  | [required] |
**name** | Option<**String**> |  |  |
**key** | Option<**String**> |  |  |
**actor_ids** | Option<**String**> | Deprecated. |  |
**actor_id** | Option<[**Vec<String>**](String.md)> |  |  |
**include_destroyed** | Option<**bool**> |  |  |
**limit** | Option<**i32**> |  |  |
**cursor** | Option<**String**> |  |  |

### Return type

[**models::ActorsListResponse**](ActorsListResponse.md)

### Authorization

[bearer_auth](../README.md#bearer_auth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

