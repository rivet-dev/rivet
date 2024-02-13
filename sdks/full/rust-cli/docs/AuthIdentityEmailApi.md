# \AuthIdentityEmailApi

All URIs are relative to *https://api.rivet.gg*

Method | HTTP request | Description
------------- | ------------- | -------------
[**auth_identity_email_complete_email_verification**](AuthIdentityEmailApi.md#auth_identity_email_complete_email_verification) | **POST** /auth/identity/email/complete-verification | 
[**auth_identity_email_start_email_verification**](AuthIdentityEmailApi.md#auth_identity_email_start_email_verification) | **POST** /auth/identity/email/start-verification | 



## auth_identity_email_complete_email_verification

> crate::models::AuthIdentityCompleteEmailVerificationResponse auth_identity_email_complete_email_verification(auth_identity_complete_email_verification_request)


Completes the email verification process.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**auth_identity_complete_email_verification_request** | [**AuthIdentityCompleteEmailVerificationRequest**](AuthIdentityCompleteEmailVerificationRequest.md) |  | [required] |

### Return type

[**crate::models::AuthIdentityCompleteEmailVerificationResponse**](AuthIdentityCompleteEmailVerificationResponse.md)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## auth_identity_email_start_email_verification

> crate::models::AuthIdentityStartEmailVerificationResponse auth_identity_email_start_email_verification(auth_identity_start_email_verification_request)


Starts the verification process for linking an email to your identity.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**auth_identity_start_email_verification_request** | [**AuthIdentityStartEmailVerificationRequest**](AuthIdentityStartEmailVerificationRequest.md) |  | [required] |

### Return type

[**crate::models::AuthIdentityStartEmailVerificationResponse**](AuthIdentityStartEmailVerificationResponse.md)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

