# \RoutesApi

All URIs are relative to *https://api.rivet.gg*

Method | HTTP request | Description
------------- | ------------- | -------------
[**routes_delete**](RoutesApi.md#routes_delete) | **DELETE** /routes/{id} | 
[**routes_history**](RoutesApi.md#routes_history) | **GET** /routes/history | 
[**routes_list**](RoutesApi.md#routes_list) | **GET** /routes | 
[**routes_update**](RoutesApi.md#routes_update) | **PUT** /routes/{id} | 



## routes_delete

> serde_json::Value routes_delete(id, project, environment)


Deletes a route.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**id** | **String** |  | [required] |
**project** | Option<**String**> |  |  |
**environment** | Option<**String**> |  |  |

### Return type

[**serde_json::Value**](serde_json::Value.md)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## routes_history

> crate::models::RoutesHistoryResponse routes_history(start, end, interval, project, environment, query_json, group_by)


Returns time series data for HTTP requests routed to actors. Allows filtering and grouping by various request properties.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**start** | **i32** | Start timestamp in milliseconds | [required] |
**end** | **i32** | End timestamp in milliseconds | [required] |
**interval** | **i32** | Time bucket interval in milliseconds | [required] |
**project** | Option<**String**> |  |  |
**environment** | Option<**String**> |  |  |
**query_json** | Option<**String**> | JSON-encoded query expression for filtering requests |  |
**group_by** | Option<**String**> | JSON-encoded KeyPath for grouping results (e.g. {\"property\":\"client_request_host\"} or {\"property\":\"tags\",\"map_key\":\"version\"}) |  |

### Return type

[**crate::models::RoutesHistoryResponse**](RoutesHistoryResponse.md)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## routes_list

> crate::models::RoutesListRoutesResponse routes_list(project, environment)


Lists all routes of the given environment.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**project** | Option<**String**> |  |  |
**environment** | Option<**String**> |  |  |

### Return type

[**crate::models::RoutesListRoutesResponse**](RoutesListRoutesResponse.md)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## routes_update

> serde_json::Value routes_update(id, routes_update_route_body, project, environment)


Creates or updates a route.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**id** | **String** |  | [required] |
**routes_update_route_body** | [**RoutesUpdateRouteBody**](RoutesUpdateRouteBody.md) |  | [required] |
**project** | Option<**String**> |  |  |
**environment** | Option<**String**> |  |  |

### Return type

[**serde_json::Value**](serde_json::Value.md)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

