# \PartyPartiesApi

All URIs are relative to *http://localhost*

Method | HTTP request | Description
------------- | ------------- | -------------
[**party_parties_create**](PartyPartiesApi.md#party_parties_create) | **POST** /parties | 
[**party_parties_create_invite**](PartyPartiesApi.md#party_parties_create_invite) | **POST** /parties/self/invites | 
[**party_parties_get_from_invite**](PartyPartiesApi.md#party_parties_get_from_invite) | **GET** /invites | 
[**party_parties_get_profile**](PartyPartiesApi.md#party_parties_get_profile) | **GET** /parties/{party_id}/profile | 
[**party_parties_get_self_profile**](PartyPartiesApi.md#party_parties_get_self_profile) | **GET** /parties/self/profile | 
[**party_parties_get_self_summary**](PartyPartiesApi.md#party_parties_get_self_summary) | **GET** /parties/self/summary | 
[**party_parties_get_summary**](PartyPartiesApi.md#party_parties_get_summary) | **GET** /parties/{party_id}/summary | 
[**party_parties_join**](PartyPartiesApi.md#party_parties_join) | **POST** /parties/join | 
[**party_parties_kick_member**](PartyPartiesApi.md#party_parties_kick_member) | **POST** /parties/self/members/{identity_id}/kick | 
[**party_parties_leave**](PartyPartiesApi.md#party_parties_leave) | **POST** /parties/self/leave | 
[**party_parties_revoke_invite**](PartyPartiesApi.md#party_parties_revoke_invite) | **DELETE** /parties/self/invites/{invite_id} | 
[**party_parties_send_join_request**](PartyPartiesApi.md#party_parties_send_join_request) | **POST** /parties/{party_id}/join-request/send | 
[**party_parties_set_publicity**](PartyPartiesApi.md#party_parties_set_publicity) | **PUT** /parties/self/publicity | 
[**party_parties_transfer_ownership**](PartyPartiesApi.md#party_parties_transfer_ownership) | **POST** /parties/self/members/{identity_id}/transfer-ownership | 



## party_parties_create

> crate::models::PartyCreateResponse party_parties_create(party_create_request)


Creates a new party.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**party_create_request** | [**PartyCreateRequest**](PartyCreateRequest.md) |  | [required] |

### Return type

[**crate::models::PartyCreateResponse**](PartyCreateResponse.md)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## party_parties_create_invite

> crate::models::PartyCreateInviteResponse party_parties_create_invite(party_create_invite_request)


Creates a new party invite for the current identity's party. Identity must be the party leader.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**party_create_invite_request** | [**PartyCreateInviteRequest**](PartyCreateInviteRequest.md) |  | [required] |

### Return type

[**crate::models::PartyCreateInviteResponse**](PartyCreateInviteResponse.md)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## party_parties_get_from_invite

> crate::models::PartyGetInviteResponse party_parties_get_from_invite(token, alias)


Fetches a party based on a given invite.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**token** | Option<**String**> | See `rivet.api.party#CreatedInvite$token`. |  |
**alias** | Option<**String**> | An alias used to join a given party. This alias must be unique for all invites for your game. Pass this alias to `rivet.api.party.common#CreatedInvite$alias` to consume the invite. |  |

### Return type

[**crate::models::PartyGetInviteResponse**](PartyGetInviteResponse.md)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## party_parties_get_profile

> crate::models::PartyGetProfileResponse party_parties_get_profile(party_id, watch_index)


Returns a party profile.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**party_id** | **uuid::Uuid** |  | [required] |
**watch_index** | Option<**String**> | A query parameter denoting the requests watch index. |  |

### Return type

[**crate::models::PartyGetProfileResponse**](PartyGetProfileResponse.md)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## party_parties_get_self_profile

> crate::models::PartyGetSelfProfileResponse party_parties_get_self_profile(watch_index)


Returns a party profile for the party the current identity is a member of.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**watch_index** | Option<**String**> | A query parameter denoting the requests watch index. |  |

### Return type

[**crate::models::PartyGetSelfProfileResponse**](PartyGetSelfProfileResponse.md)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## party_parties_get_self_summary

> crate::models::PartyGetSelfSummaryResponse party_parties_get_self_summary(watch_index)


Returns a party summary for the party the current identity is a member of.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**watch_index** | Option<**String**> | A query parameter denoting the requests watch index. |  |

### Return type

[**crate::models::PartyGetSelfSummaryResponse**](PartyGetSelfSummaryResponse.md)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## party_parties_get_summary

> crate::models::PartyGetSummaryResponse party_parties_get_summary(party_id, watch_index)


Returns a party summary.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**party_id** | **uuid::Uuid** |  | [required] |
**watch_index** | Option<**String**> | A query parameter denoting the requests watch index. |  |

### Return type

[**crate::models::PartyGetSummaryResponse**](PartyGetSummaryResponse.md)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## party_parties_join

> crate::models::PartyJoinResponse party_parties_join(party_join_request)


Joins a party using a given party invite.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**party_join_request** | [**PartyJoinRequest**](PartyJoinRequest.md) |  | [required] |

### Return type

[**crate::models::PartyJoinResponse**](PartyJoinResponse.md)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## party_parties_kick_member

> party_parties_kick_member(identity_id)


Kicks a member from the current identity's current party. Identity must be the party leader.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**identity_id** | **uuid::Uuid** |  | [required] |

### Return type

 (empty response body)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## party_parties_leave

> party_parties_leave()


Leaves the current identity's party.

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


## party_parties_revoke_invite

> party_parties_revoke_invite(invite_id)


Revokes a party invite from the current identity's party. Identity must be the party leader.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**invite_id** | **uuid::Uuid** |  | [required] |

### Return type

 (empty response body)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## party_parties_send_join_request

> party_parties_send_join_request(party_id)


### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**party_id** | **uuid::Uuid** |  | [required] |

### Return type

 (empty response body)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## party_parties_set_publicity

> party_parties_set_publicity(party_set_publicity_request)


Sets the publicity of a party. This configures who can view and join the party. Identity must be the party leader.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**party_set_publicity_request** | [**PartySetPublicityRequest**](PartySetPublicityRequest.md) |  | [required] |

### Return type

 (empty response body)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## party_parties_transfer_ownership

> party_parties_transfer_ownership(identity_id)


Transfers ownership of the party to another party member. Identity must be the party leader.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**identity_id** | **uuid::Uuid** |  | [required] |

### Return type

 (empty response body)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

