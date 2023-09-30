# \ChatIdentityApi

All URIs are relative to *https://api.rivet.gg*

Method | HTTP request | Description
------------- | ------------- | -------------
[**chat_identity_get_direct_thread**](ChatIdentityApi.md#chat_identity_get_direct_thread) | **GET** /chat/identities/{identity_id}/thread | 



## chat_identity_get_direct_thread

> crate::models::ChatGetDirectThreadResponse chat_identity_get_direct_thread(identity_id)


Returns a thread ID with a given identity.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**identity_id** | **uuid::Uuid** |  | [required] |

### Return type

[**crate::models::ChatGetDirectThreadResponse**](ChatGetDirectThreadResponse.md)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

