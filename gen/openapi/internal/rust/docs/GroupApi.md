# \GroupApi

All URIs are relative to *http://localhost*

Method | HTTP request | Description
------------- | ------------- | -------------
[**group_ban_identity**](GroupApi.md#group_ban_identity) | **POST** /groups/{group_id}/bans/{identity_id} | 
[**group_complete_avatar_upload**](GroupApi.md#group_complete_avatar_upload) | **POST** /groups/{group_id}/avatar-upload/{upload_id}/complete | 
[**group_create**](GroupApi.md#group_create) | **POST** /groups | 
[**group_get_bans**](GroupApi.md#group_get_bans) | **GET** /groups/{group_id}/bans | 
[**group_get_join_requests**](GroupApi.md#group_get_join_requests) | **GET** /groups/{group_id}/join-requests | 
[**group_get_members**](GroupApi.md#group_get_members) | **GET** /groups/{group_id}/members | 
[**group_get_profile**](GroupApi.md#group_get_profile) | **GET** /groups/{group_id}/profile | 
[**group_get_summary**](GroupApi.md#group_get_summary) | **GET** /groups/{group_id}/summary | 
[**group_kick_member**](GroupApi.md#group_kick_member) | **POST** /groups/{group_id}/kick/{identity_id} | 
[**group_leave**](GroupApi.md#group_leave) | **POST** /groups/{group_id}/leave | 
[**group_list_suggested**](GroupApi.md#group_list_suggested) | **GET** /groups | 
[**group_prepare_avatar_upload**](GroupApi.md#group_prepare_avatar_upload) | **POST** /groups/avatar-upload/prepare | 
[**group_search**](GroupApi.md#group_search) | **GET** /groups/search | 
[**group_transfer_ownership**](GroupApi.md#group_transfer_ownership) | **POST** /groups/{group_id}/transfer-owner | 
[**group_unban_identity**](GroupApi.md#group_unban_identity) | **DELETE** /groups/{group_id}/bans/{identity_id} | 
[**group_update_profile**](GroupApi.md#group_update_profile) | **POST** /groups/{group_id}/profile | 
[**group_validate_profile**](GroupApi.md#group_validate_profile) | **POST** /groups/profile/validate | 



## group_ban_identity

> group_ban_identity(group_id, identity_id)


Bans an identity from a group. Must be the owner of the group to perform this action. The banned identity will no longer be able to create a join request or use a group invite.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**group_id** | **uuid::Uuid** |  | [required] |
**identity_id** | **uuid::Uuid** |  | [required] |

### Return type

 (empty response body)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## group_complete_avatar_upload

> group_complete_avatar_upload(group_id, upload_id)


Completes an avatar image upload. Must be called after the file upload process completes. Call `rivet.api.group#PrepareAvatarUpload` first.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**group_id** | **uuid::Uuid** |  | [required] |
**upload_id** | **uuid::Uuid** |  | [required] |

### Return type

 (empty response body)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## group_create

> crate::models::GroupCreateResponse group_create(group_create_request)


Creates a new group.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**group_create_request** | [**GroupCreateRequest**](GroupCreateRequest.md) |  | [required] |

### Return type

[**crate::models::GroupCreateResponse**](GroupCreateResponse.md)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## group_get_bans

> crate::models::GroupGetBansResponse group_get_bans(group_id, anchor, count, watch_index)


Returns a group's bans. Must have valid permissions to view.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**group_id** | **uuid::Uuid** |  | [required] |
**anchor** | Option<**String**> | The pagination anchor. Set to the returned anchor of this endpoint to receive the next set of items. |  |
**count** | Option<**f64**> | Amount of bans to return. |  |
**watch_index** | Option<**String**> | A query parameter denoting the requests watch index. |  |

### Return type

[**crate::models::GroupGetBansResponse**](GroupGetBansResponse.md)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## group_get_join_requests

> crate::models::GroupGetJoinRequestsResponse group_get_join_requests(group_id, anchor, count, watch_index)


Returns a group's join requests. Must have valid permissions to view.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**group_id** | **uuid::Uuid** |  | [required] |
**anchor** | Option<**String**> | The pagination anchor. Set to the returned anchor of this endpoint to receive the next set of items. |  |
**count** | Option<**f64**> | Amount of join requests to return. |  |
**watch_index** | Option<**String**> | A query parameter denoting the requests watch index. |  |

### Return type

[**crate::models::GroupGetJoinRequestsResponse**](GroupGetJoinRequestsResponse.md)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## group_get_members

> crate::models::GroupGetMembersResponse group_get_members(group_id, anchor, count, watch_index)


Returns a group's members.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**group_id** | **uuid::Uuid** |  | [required] |
**anchor** | Option<**String**> | The pagination anchor. Set to the returned anchor of this endpoint to receive the next set of items. |  |
**count** | Option<**f64**> | Amount of members to return. |  |
**watch_index** | Option<**String**> | A query parameter denoting the requests watch index. |  |

### Return type

[**crate::models::GroupGetMembersResponse**](GroupGetMembersResponse.md)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## group_get_profile

> crate::models::GroupGetProfileResponse group_get_profile(group_id, watch_index)


Returns a group profile.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**group_id** | **uuid::Uuid** |  | [required] |
**watch_index** | Option<**String**> | A query parameter denoting the requests watch index. |  |

### Return type

[**crate::models::GroupGetProfileResponse**](GroupGetProfileResponse.md)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## group_get_summary

> crate::models::GroupGetSummaryResponse group_get_summary(group_id)


### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**group_id** | **uuid::Uuid** |  | [required] |

### Return type

[**crate::models::GroupGetSummaryResponse**](GroupGetSummaryResponse.md)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## group_kick_member

> group_kick_member(group_id, identity_id)


Kicks an identity from a group. Must be the owner of the group to perform this action.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**group_id** | **uuid::Uuid** |  | [required] |
**identity_id** | **uuid::Uuid** |  | [required] |

### Return type

 (empty response body)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## group_leave

> group_leave(group_id)


Leaves a group.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**group_id** | **uuid::Uuid** |  | [required] |

### Return type

 (empty response body)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## group_list_suggested

> crate::models::GroupListSuggestedResponse group_list_suggested(watch_index)


Returns a list of suggested groups.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**watch_index** | Option<**String**> | A query parameter denoting the requests watch index. |  |

### Return type

[**crate::models::GroupListSuggestedResponse**](GroupListSuggestedResponse.md)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## group_prepare_avatar_upload

> crate::models::GroupPrepareAvatarUploadResponse group_prepare_avatar_upload(group_prepare_avatar_upload_request)


Prepares an avatar image upload. Complete upload with `rivet.api.group#CompleteAvatarUpload`.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**group_prepare_avatar_upload_request** | [**GroupPrepareAvatarUploadRequest**](GroupPrepareAvatarUploadRequest.md) |  | [required] |

### Return type

[**crate::models::GroupPrepareAvatarUploadResponse**](GroupPrepareAvatarUploadResponse.md)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## group_search

> crate::models::GroupSearchResponse group_search(query, anchor, limit)


Fuzzy search for groups.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**query** | **String** | The query to match group display names against. | [required] |
**anchor** | Option<**String**> |  |  |
**limit** | Option<**f64**> | Unsigned 32 bit integer. |  |

### Return type

[**crate::models::GroupSearchResponse**](GroupSearchResponse.md)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## group_transfer_ownership

> group_transfer_ownership(group_id, group_transfer_ownership_request)


Transfers ownership of a group to another identity.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**group_id** | **uuid::Uuid** |  | [required] |
**group_transfer_ownership_request** | [**GroupTransferOwnershipRequest**](GroupTransferOwnershipRequest.md) |  | [required] |

### Return type

 (empty response body)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## group_unban_identity

> group_unban_identity(group_id, identity_id)


Unbans an identity from a group. Must be the owner of the group to perform this action.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**group_id** | **uuid::Uuid** |  | [required] |
**identity_id** | **uuid::Uuid** |  | [required] |

### Return type

 (empty response body)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## group_update_profile

> group_update_profile(group_id, group_update_profile_request)


### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**group_id** | **uuid::Uuid** |  | [required] |
**group_update_profile_request** | [**GroupUpdateProfileRequest**](GroupUpdateProfileRequest.md) |  | [required] |

### Return type

 (empty response body)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## group_validate_profile

> crate::models::GroupValidateProfileResponse group_validate_profile(group_validate_profile_request)


Validate contents of group profile. Use to provide immediate feedback on profile changes before committing them.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**group_validate_profile_request** | [**GroupValidateProfileRequest**](GroupValidateProfileRequest.md) |  | [required] |

### Return type

[**crate::models::GroupValidateProfileResponse**](GroupValidateProfileResponse.md)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

