# \EdgeIntercomPegboardApi

All URIs are relative to *https://api.rivet.gg*

Method | HTTP request | Description
------------- | ------------- | -------------
[**edge_intercom_pegboard_prewarm_image**](EdgeIntercomPegboardApi.md#edge_intercom_pegboard_prewarm_image) | **POST** /pegboard/image/{image_id}/prewarm | 
[**edge_intercom_pegboard_toggle_client_drain**](EdgeIntercomPegboardApi.md#edge_intercom_pegboard_toggle_client_drain) | **POST** /pegboard/client/{client_id}/toggle-drain | 



## edge_intercom_pegboard_prewarm_image

> edge_intercom_pegboard_prewarm_image(image_id, edge_intercom_pegboard_prewarm_image_request)


### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**image_id** | **uuid::Uuid** |  | [required] |
**edge_intercom_pegboard_prewarm_image_request** | [**EdgeIntercomPegboardPrewarmImageRequest**](EdgeIntercomPegboardPrewarmImageRequest.md) |  | [required] |

### Return type

 (empty response body)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## edge_intercom_pegboard_toggle_client_drain

> edge_intercom_pegboard_toggle_client_drain(client_id, edge_intercom_pegboard_toggle_client_drain_request)


### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**client_id** | **uuid::Uuid** |  | [required] |
**edge_intercom_pegboard_toggle_client_drain_request** | [**EdgeIntercomPegboardToggleClientDrainRequest**](EdgeIntercomPegboardToggleClientDrainRequest.md) |  | [required] |

### Return type

 (empty response body)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

