# \IdentityLinksApi

All URIs are relative to *https://api.rivet.gg*

Method | HTTP request | Description
------------- | ------------- | -------------
[**identity_links_cancel**](IdentityLinksApi.md#identity_links_cancel) | **POST** /identity/game-links/cancel | 
[**identity_links_complete**](IdentityLinksApi.md#identity_links_complete) | **POST** /identity/game-links/complete | 
[**identity_links_get**](IdentityLinksApi.md#identity_links_get) | **GET** /identity/game-links | 
[**identity_links_prepare**](IdentityLinksApi.md#identity_links_prepare) | **POST** /identity/game-links | 



## identity_links_cancel

> identity_links_cancel(identity_cancel_game_link_request)


Cancels a game link. It can no longer be used to link after cancellation.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**identity_cancel_game_link_request** | [**IdentityCancelGameLinkRequest**](IdentityCancelGameLinkRequest.md) |  | [required] |

### Return type

 (empty response body)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## identity_links_complete

> identity_links_complete(identity_complete_game_link_request)


Completes a game link process and returns whether or not the link is valid.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**identity_complete_game_link_request** | [**IdentityCompleteGameLinkRequest**](IdentityCompleteGameLinkRequest.md) |  | [required] |

### Return type

 (empty response body)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## identity_links_get

> crate::models::IdentityGetGameLinkResponse identity_links_get(identity_link_token, watch_index)


Returns the current status of a linking process. Once `status` is `complete`, the identity's profile should be fetched again since they may have switched accounts.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**identity_link_token** | **String** |  | [required] |
**watch_index** | Option<**String**> |  |  |

### Return type

[**crate::models::IdentityGetGameLinkResponse**](IdentityGetGameLinkResponse.md)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## identity_links_prepare

> crate::models::IdentityPrepareGameLinkResponse identity_links_prepare()


Begins the process for linking an identity with the Rivet Hub.  # Importance of Linking Identities  When an identity is created via `rivet.api.identity#SetupIdentity`, the identity is temporary and is not shared with other games the user plays. In order to make the identity permanent and synchronize the identity with other games, the identity must be linked with the hub.  # Linking Process  The linking process works by opening `identity_link_url` in a browser then polling `rivet.api.identity#GetGameLink` to wait for it to complete. This is designed to be as flexible as possible so `identity_link_url` can be opened on any device. For example, when playing a console game, the user can scan a QR code for `identity_link_url` to authenticate on their phone.

### Parameters

This endpoint does not need any parameter.

### Return type

[**crate::models::IdentityPrepareGameLinkResponse**](IdentityPrepareGameLinkResponse.md)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

