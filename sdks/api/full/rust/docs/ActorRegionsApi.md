# \ActorRegionsApi

All URIs are relative to *https://api.rivet.gg*

Method | HTTP request | Description
------------- | ------------- | -------------
[**actor_regions_list**](ActorRegionsApi.md#actor_regions_list) | **GET** /regions | 
[**actor_regions_resolve**](ActorRegionsApi.md#actor_regions_resolve) | **GET** /regions/resolve | 



## actor_regions_list

> crate::models::ActorListRegionsResponse actor_regions_list(project, environment)


### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**project** | Option<**String**> |  |  |
**environment** | Option<**String**> |  |  |

### Return type

[**crate::models::ActorListRegionsResponse**](ActorListRegionsResponse.md)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## actor_regions_resolve

> crate::models::ActorResolveRegionResponse actor_regions_resolve(long, lat)


### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**long** | Option<**f64**> |  |  |
**lat** | Option<**f64**> |  |  |

### Return type

[**crate::models::ActorResolveRegionResponse**](ActorResolveRegionResponse.md)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

