# \IdentityApi

All URIs are relative to *https://api.rivet.gg*

Method | HTTP request | Description
------------- | ------------- | -------------
[**identity_complete_avatar_upload**](IdentityApi.md#identity_complete_avatar_upload) | **POST** /identity/identities/avatar-upload/{upload_id}/complete | 
[**identity_get_handles**](IdentityApi.md#identity_get_handles) | **GET** /identity/identities/batch/handle | 
[**identity_get_profile**](IdentityApi.md#identity_get_profile) | **GET** /identity/identities/{identity_id}/profile | 
[**identity_get_self_profile**](IdentityApi.md#identity_get_self_profile) | **GET** /identity/identities/self/profile | 
[**identity_get_summaries**](IdentityApi.md#identity_get_summaries) | **GET** /identity/identities/batch/summary | 
[**identity_mark_deletion**](IdentityApi.md#identity_mark_deletion) | **POST** /identity/identities/self/delete-request | 
[**identity_prepare_avatar_upload**](IdentityApi.md#identity_prepare_avatar_upload) | **POST** /identity/identities/avatar-upload/prepare | 
[**identity_remove_game_activity**](IdentityApi.md#identity_remove_game_activity) | **DELETE** /identity/identities/self/activity | 
[**identity_set_game_activity**](IdentityApi.md#identity_set_game_activity) | **POST** /identity/identities/self/activity | 
[**identity_setup**](IdentityApi.md#identity_setup) | **POST** /identity/identities | 
[**identity_signup_for_beta**](IdentityApi.md#identity_signup_for_beta) | **POST** /identity/identities/self/beta-signup | 
[**identity_unmark_deletion**](IdentityApi.md#identity_unmark_deletion) | **DELETE** /identity/identities/self/delete-request | 
[**identity_update_profile**](IdentityApi.md#identity_update_profile) | **POST** /identity/identities/self/profile | 
[**identity_update_status**](IdentityApi.md#identity_update_status) | **POST** /identity/identities/identities/self/status | 
[**identity_validate_profile**](IdentityApi.md#identity_validate_profile) | **POST** /identity/identities/self/profile/validate | 



## identity_complete_avatar_upload

> identity_complete_avatar_upload(upload_id)


Completes an avatar image upload. Must be called after the file upload process completes.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**upload_id** | **uuid::Uuid** |  | [required] |

### Return type

 (empty response body)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## identity_get_handles

> crate::models::IdentityGetHandlesResponse identity_get_handles(identity_ids)


Fetches a list of identity handles.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**identity_ids** | **String** |  | [required] |

### Return type

[**crate::models::IdentityGetHandlesResponse**](IdentityGetHandlesResponse.md)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## identity_get_profile

> crate::models::IdentityGetProfileResponse identity_get_profile(identity_id, watch_index)


Fetches an identity profile.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**identity_id** | **uuid::Uuid** |  | [required] |
**watch_index** | Option<**String**> |  |  |

### Return type

[**crate::models::IdentityGetProfileResponse**](IdentityGetProfileResponse.md)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## identity_get_self_profile

> crate::models::IdentityGetProfileResponse identity_get_self_profile(watch_index)


Fetches the current identity's profile.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**watch_index** | Option<**String**> |  |  |

### Return type

[**crate::models::IdentityGetProfileResponse**](IdentityGetProfileResponse.md)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## identity_get_summaries

> crate::models::IdentityGetSummariesResponse identity_get_summaries(identity_ids)


Fetches a list of identity summaries.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**identity_ids** | **String** |  | [required] |

### Return type

[**crate::models::IdentityGetSummariesResponse**](IdentityGetSummariesResponse.md)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## identity_mark_deletion

> identity_mark_deletion()


### Parameters

This endpoint does not need any parameter.

### Return type

 (empty response body)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## identity_prepare_avatar_upload

> crate::models::IdentityPrepareAvatarUploadResponse identity_prepare_avatar_upload(identity_prepare_avatar_upload_request)


Prepares an avatar image upload. Complete upload with `CompleteIdentityAvatarUpload`.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**identity_prepare_avatar_upload_request** | [**IdentityPrepareAvatarUploadRequest**](IdentityPrepareAvatarUploadRequest.md) |  | [required] |

### Return type

[**crate::models::IdentityPrepareAvatarUploadResponse**](IdentityPrepareAvatarUploadResponse.md)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## identity_remove_game_activity

> identity_remove_game_activity()


Removes the current identity's game activity.

### Parameters

This endpoint does not need any parameter.

### Return type

 (empty response body)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## identity_set_game_activity

> identity_set_game_activity(identity_set_game_activity_request)


Sets the current identity's game activity. This activity will automatically be removed when the identity goes offline.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**identity_set_game_activity_request** | [**IdentitySetGameActivityRequest**](IdentitySetGameActivityRequest.md) |  | [required] |

### Return type

 (empty response body)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## identity_setup

> crate::models::IdentitySetupResponse identity_setup(identity_setup_request)


Gets or creates an identity. Passing an existing identity token in the body refreshes the token. Temporary Accounts Until the identity is linked with the Rivet Hub (see `PrepareGameLink`), this identity will be temporary but still behave like all other identities. This is intended to allow users to play the game without signing up while still having the benefits of having an account. When they are ready to save their account, they should be instructed to link their account (see `PrepareGameLink`). Storing Token `identity_token` should be stored in some form of persistent storage. The token should be read from storage and passed to `Setup` every time the client starts.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**identity_setup_request** | [**IdentitySetupRequest**](IdentitySetupRequest.md) |  | [required] |

### Return type

[**crate::models::IdentitySetupResponse**](IdentitySetupResponse.md)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## identity_signup_for_beta

> identity_signup_for_beta(identity_signup_for_beta_request)


Completes an avatar image upload. Must be called after the file upload process completes.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**identity_signup_for_beta_request** | [**IdentitySignupForBetaRequest**](IdentitySignupForBetaRequest.md) |  | [required] |

### Return type

 (empty response body)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## identity_unmark_deletion

> identity_unmark_deletion()


### Parameters

This endpoint does not need any parameter.

### Return type

 (empty response body)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## identity_update_profile

> identity_update_profile(identity_update_profile_request)


Updates profile of the current identity.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**identity_update_profile_request** | [**IdentityUpdateProfileRequest**](IdentityUpdateProfileRequest.md) |  | [required] |

### Return type

 (empty response body)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## identity_update_status

> identity_update_status(identity_update_status_request)


Updates the current identity's status.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**identity_update_status_request** | [**IdentityUpdateStatusRequest**](IdentityUpdateStatusRequest.md) |  | [required] |

### Return type

 (empty response body)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## identity_validate_profile

> identity_validate_profile(identity_update_profile_request)


Validate contents of identity profile. Use to provide immediate feedback on profile changes before committing them.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**identity_update_profile_request** | [**IdentityUpdateProfileRequest**](IdentityUpdateProfileRequest.md) |  | [required] |

### Return type

 (empty response body)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

