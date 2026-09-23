# \AuthTokensApi

All URIs are relative to *http://localhost*

Method | HTTP request | Description
------------- | ------------- | -------------
[**auth_tokens_create**](AuthTokensApi.md#auth_tokens_create) | **POST** /auth/tokens | 
[**auth_tokens_inspect**](AuthTokensApi.md#auth_tokens_inspect) | **GET** /auth/tokens/inspect | 



## auth_tokens_create

> models::AuthTokenCreateResponse auth_tokens_create(auth_token_create_request)


### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**auth_token_create_request** | [**AuthTokenCreateRequest**](AuthTokenCreateRequest.md) |  | [required] |

### Return type

[**models::AuthTokenCreateResponse**](AuthTokenCreateResponse.md)

### Authorization

[bearer_auth](../README.md#bearer_auth)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## auth_tokens_inspect

> models::AuthTokenInspectResponse auth_tokens_inspect()


### Parameters

This endpoint does not need any parameter.

### Return type

[**models::AuthTokenInspectResponse**](AuthTokenInspectResponse.md)

### Authorization

[bearer_auth](../README.md#bearer_auth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

