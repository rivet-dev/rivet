# \RegionsApi

All URIs are relative to *https://api.rivet.gg*

Method | HTTP request | Description
------------- | ------------- | -------------
[**regions_list**](RegionsApi.md#regions_list) | **GET** /regions | 
[**regions_recommend**](RegionsApi.md#regions_recommend) | **GET** /regions/recommend | 



## regions_list

> crate::models::RegionsListRegionsResponse regions_list(project, environment)


### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**project** | Option<**String**> |  |  |
**environment** | Option<**String**> |  |  |

### Return type

[**crate::models::RegionsListRegionsResponse**](RegionsListRegionsResponse.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## regions_recommend

> crate::models::RegionsRecommendRegionResponse regions_recommend(project, environment, lat, long)


### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**project** | Option<**String**> |  |  |
**environment** | Option<**String**> |  |  |
**lat** | Option<**f64**> |  |  |
**long** | Option<**f64**> |  |  |

### Return type

[**crate::models::RegionsRecommendRegionResponse**](RegionsRecommendRegionResponse.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

