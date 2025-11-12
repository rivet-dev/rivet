# \RunnersApi

All URIs are relative to *http://localhost*

Method | HTTP request | Description
------------- | ------------- | -------------
[**runners_list**](RunnersApi.md#runners_list) | **GET** /runners | 
[**runners_list_names**](RunnersApi.md#runners_list_names) | **GET** /runners/names | ## Datacenter Round Trips



## runners_list

> models::RunnersListResponse runners_list(namespace, name, runner_ids, runner_id, include_stopped, limit, cursor)


### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**namespace** | **String** |  | [required] |
**name** | Option<**String**> |  |  |
**runner_ids** | Option<**String**> | Deprecated. |  |
**runner_id** | Option<[**Vec<String>**](String.md)> |  |  |
**include_stopped** | Option<**bool**> |  |  |
**limit** | Option<**i32**> |  |  |
**cursor** | Option<**String**> |  |  |

### Return type

[**models::RunnersListResponse**](RunnersListResponse.md)

### Authorization

[bearer_auth](../README.md#bearer_auth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## runners_list_names

> models::RunnersListNamesResponse runners_list_names(namespace, limit, cursor)
## Datacenter Round Trips

2 round trips: - GET /runners/names (fanout) - [api-peer] namespace::ops::resolve_for_name_global

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**namespace** | **String** |  | [required] |
**limit** | Option<**i32**> |  |  |
**cursor** | Option<**String**> |  |  |

### Return type

[**models::RunnersListNamesResponse**](RunnersListNamesResponse.md)

### Authorization

[bearer_auth](../README.md#bearer_auth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

