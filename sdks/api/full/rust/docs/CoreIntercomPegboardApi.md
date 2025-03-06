# \CoreIntercomPegboardApi

All URIs are relative to *https://api.rivet.gg*

Method | HTTP request | Description
------------- | ------------- | -------------
[**core_intercom_pegboard_mark_client_registered**](CoreIntercomPegboardApi.md#core_intercom_pegboard_mark_client_registered) | **POST** /pegboard/client/{client_id}/registered | 



## core_intercom_pegboard_mark_client_registered

> core_intercom_pegboard_mark_client_registered(client_id, core_intercom_pegboard_mark_client_registered_request)


### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**client_id** | **uuid::Uuid** |  | [required] |
**core_intercom_pegboard_mark_client_registered_request** | [**CoreIntercomPegboardMarkClientRegisteredRequest**](CoreIntercomPegboardMarkClientRegisteredRequest.md) |  | [required] |

### Return type

 (empty response body)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

