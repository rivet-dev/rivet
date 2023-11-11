# \GroupInvitesApi

All URIs are relative to *https://api.rivet.gg*

Method | HTTP request | Description
------------- | ------------- | -------------
[**group_invites_consume_invite**](GroupInvitesApi.md#group_invites_consume_invite) | **POST** /group/invites/{group_invite_code}/consume | 
[**group_invites_create_invite**](GroupInvitesApi.md#group_invites_create_invite) | **POST** /group/groups/{group_id}/invites | 
[**group_invites_get_invite**](GroupInvitesApi.md#group_invites_get_invite) | **GET** /group/invites/{group_invite_code} | 



## group_invites_consume_invite

> crate::models::GroupConsumeInviteResponse group_invites_consume_invite(group_invite_code)


Consumes a group invite to join a group.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**group_invite_code** | **String** | Provided by `rivet.api.group#CreateInviteResponse$code`. | [required] |

### Return type

[**crate::models::GroupConsumeInviteResponse**](GroupConsumeInviteResponse.md)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## group_invites_create_invite

> crate::models::GroupCreateInviteResponse group_invites_create_invite(group_id, group_create_invite_request)


Creates a group invite. Can be shared with other identities to let them join this group.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**group_id** | **uuid::Uuid** |  | [required] |
**group_create_invite_request** | [**GroupCreateInviteRequest**](GroupCreateInviteRequest.md) |  | [required] |

### Return type

[**crate::models::GroupCreateInviteResponse**](GroupCreateInviteResponse.md)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## group_invites_get_invite

> crate::models::GroupGetInviteResponse group_invites_get_invite(group_invite_code)


Inspects a group invite returning information about the team that created it.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**group_invite_code** | **String** | Provided by `rivet.api.group#CreateInviteResponse$code`. | [required] |

### Return type

[**crate::models::GroupGetInviteResponse**](GroupGetInviteResponse.md)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

