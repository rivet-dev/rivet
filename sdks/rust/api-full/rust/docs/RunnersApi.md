# \RunnersApi

All URIs are relative to *http://localhost*

Method | HTTP request | Description
------------- | ------------- | -------------
[**runners_get**](RunnersApi.md#runners_get) | **GET** /runners/{runner_id} | 
[**runners_list**](RunnersApi.md#runners_list) | **GET** /runners | 
[**runners_list_names**](RunnersApi.md#runners_list_names) | **GET** /runners/names | ## Datacenter Round Trips



## runners_get

> models::RunnersGetResponse runners_get(runner_id, namespace)


### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**runner_id** | **String** |  | [required] |
**namespace** | Option<**String**> |  |  |

### Return type

[**models::RunnersGetResponse**](RunnersGetResponse.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## runners_list

> models::RunnersListResponse runners_list(namespace, name, include_stopped, limit, cursor)


### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**namespace** | **String** |  | [required] |
**name** | Option<**String**> |  |  |
**include_stopped** | Option<**bool**> |  |  |
**limit** | Option<**i32**> |  |  |
**cursor** | Option<**String**> |  |  |

### Return type

[**models::RunnersListResponse**](RunnersListResponse.md)

### Authorization

No authorization required

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

No authorization required

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

