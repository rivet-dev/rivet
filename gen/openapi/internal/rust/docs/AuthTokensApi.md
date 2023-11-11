# \AuthTokensApi

All URIs are relative to *https://api.rivet.gg*

Method | HTTP request | Description
------------- | ------------- | -------------
[**auth_tokens_refresh_identity_token**](AuthTokensApi.md#auth_tokens_refresh_identity_token) | **POST** /auth/tokens/identity | 



## auth_tokens_refresh_identity_token

> crate::models::AuthRefreshIdentityTokenResponse auth_tokens_refresh_identity_token(auth_refresh_identity_token_request)


Refreshes the current identity's token and sets authentication headers.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**auth_refresh_identity_token_request** | [**AuthRefreshIdentityTokenRequest**](AuthRefreshIdentityTokenRequest.md) |  | [required] |

### Return type

[**crate::models::AuthRefreshIdentityTokenResponse**](AuthRefreshIdentityTokenResponse.md)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

